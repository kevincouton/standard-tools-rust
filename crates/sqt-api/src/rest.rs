//! REST API routers.

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqt_agent::{ToolCall, ToolDefinition};
use sqt_audit::{AuditStorage, AuditVerifier};
use sqt_backtest::{BacktestConfig, BacktestService};
use sqt_core::{BarInterval, DateRange, QuantError, Result as QuantResult, Ticker};
use sqt_orders::{OrderService, OrderSide, OrderType};

use crate::state::AppState;

/// Top-level REST router.
pub fn router<S: AuditStorage + 'static>(state: Arc<AppState<S>>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/agent/tools", get(list_tools::<S>))
        .route("/api/v1/agent/dispatch", post(dispatch_tool::<S>))
        .route("/api/v1/market-data/:ticker", get(fetch_ohlcv::<S>))
        .route(
            "/api/v1/indicators/:indicator",
            post(compute_indicator::<S>),
        )
        .route("/api/v1/metrics/:metric", post(compute_metric::<S>))
        .route("/api/v1/analysis/:method", post(run_analysis::<S>))
        .route("/api/v1/backtest/:strategy", post(run_backtest::<S>))
        .route(
            "/api/v1/portfolio/mean-variance",
            post(optimize_portfolio::<S>),
        )
        .route("/api/v1/portfolio/risk-parity", post(risk_parity::<S>))
        .route(
            "/api/v1/portfolio/black-litterman",
            post(black_litterman::<S>),
        )
        .route("/api/v1/screen", post(screen::<S>))
        .route("/api/v1/audit/verify", post(audit_verify::<S>))
        .route(
            "/api/v1/orders",
            post(create_order::<S>).get(list_orders::<S>),
        )
        .route(
            "/api/v1/orders/:id",
            get(get_order::<S>)
                .post(transition_order::<S>)
                .delete(delete_order::<S>),
        )
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn list_tools<S: AuditStorage>(
    State(_state): State<Arc<AppState<S>>>,
) -> Json<Vec<ToolDefinition>> {
    Json(sqt_agent::registry::list().to_vec())
}

#[derive(Debug, Deserialize)]
struct DispatchRequest {
    tool: String,
    arguments: Value,
}

#[derive(Debug, Serialize)]
struct DispatchResponse {
    output: Value,
    error: Option<String>,
}

async fn dispatch_tool<S: AuditStorage>(
    State(state): State<Arc<AppState<S>>>,
    Json(req): Json<DispatchRequest>,
) -> Result<impl IntoResponse, AppError> {
    let call = ToolCall {
        name: req.tool.clone(),
        arguments: req.arguments.clone(),
    };

    let result = state.dispatcher.dispatch(call).await?;

    // Audit the call.
    let input = serde_json::json!({ "tool": req.tool, "arguments": req.arguments });
    let output = result.error.is_none().then_some(result.output.clone());
    let status = if result.error.is_none() {
        "ok"
    } else {
        "error"
    };
    let _ = state
        .audit_writer
        .record(
            uuid::Uuid::new_v4(),
            req.tool.clone(),
            input,
            output.as_ref(),
            status,
            result.error.clone(),
        )
        .await;

    Ok((
        StatusCode::OK,
        Json(DispatchResponse {
            output: result.output,
            error: result.error,
        }),
    ))
}

#[derive(Debug, Deserialize)]
struct OhlcvQuery {
    start: String,
    end: String,
    interval: Option<String>,
    provider: Option<String>,
}

async fn fetch_ohlcv<S: AuditStorage>(
    State(state): State<Arc<AppState<S>>>,
    Path(ticker): Path<String>,
    Query(q): Query<OhlcvQuery>,
) -> Result<impl IntoResponse, AppError> {
    let range = parse_date_range(&q.start, &q.end)?;
    let interval = parse_interval(q.interval.as_deref())?;
    let ticker = Ticker::try_new(ticker)?;
    let series = state
        .market_data
        .fetch(&ticker, range, interval, q.provider.as_deref())
        .await?;
    Ok(Json(series_to_json(&series)))
}

#[derive(Debug, Deserialize)]
struct IndicatorRequest {
    ticker: String,
    start: String,
    end: String,
    interval: Option<String>,
    provider: Option<String>,
    params: Option<HashMap<String, String>>,
}

async fn compute_indicator<S: AuditStorage>(
    State(state): State<Arc<AppState<S>>>,
    Path(indicator): Path<String>,
    Json(req): Json<IndicatorRequest>,
) -> Result<impl IntoResponse, AppError> {
    let range = parse_date_range(&req.start, &req.end)?;
    let interval = parse_interval(req.interval.as_deref())?;
    let ticker = Ticker::try_new(req.ticker)?;

    let series = state
        .market_data
        .fetch(&ticker, range, interval, req.provider.as_deref())
        .await?;
    let params = req.params.unwrap_or_default();
    let result = sqt_indicators::IndicatorCalculator::calculate(&indicator, &series, &params)?;
    Ok(Json(indicator_to_json(&result)))
}

#[derive(Debug, Deserialize)]
struct MetricRequest {
    ticker: String,
    start: String,
    end: String,
    interval: Option<String>,
    provider: Option<String>,
    risk_free_rate: Option<f64>,
    periods_per_year: Option<u32>,
    benchmark_ticker: Option<String>,
}

async fn compute_metric<S: AuditStorage>(
    State(state): State<Arc<AppState<S>>>,
    Path(metric): Path<String>,
    Json(req): Json<MetricRequest>,
) -> Result<impl IntoResponse, AppError> {
    let range = parse_date_range(&req.start, &req.end)?;
    let interval = parse_interval(req.interval.as_deref())?;
    let ticker = Ticker::try_new(req.ticker)?;

    let asset_series = state
        .market_data
        .fetch(&ticker, range, interval, req.provider.as_deref())
        .await?;
    let asset_returns = to_returns(&asset_series)?;

    let benchmark_returns = if let Some(bench) = req.benchmark_ticker {
        let bench_ticker = Ticker::try_new(bench)?;
        let bench_series = state
            .market_data
            .fetch(&bench_ticker, range, interval, req.provider.as_deref())
            .await?;
        Some(to_returns(&bench_series)?)
    } else {
        None
    };

    let result = sqt_metrics::MetricsCalculator::from_returns(
        &asset_returns,
        req.risk_free_rate.unwrap_or(0.0),
        benchmark_returns.as_deref(),
        req.periods_per_year.unwrap_or(252),
    )?;

    Ok(Json(metrics_to_json(&result, &metric)?))
}

#[derive(Debug, Deserialize)]
struct AnalysisRequest {
    arguments: Value,
}

async fn run_analysis<S: AuditStorage>(
    State(state): State<Arc<AppState<S>>>,
    Path(method): Path<String>,
    Json(req): Json<AnalysisRequest>,
) -> Result<impl IntoResponse, AppError> {
    let call = ToolCall {
        name: method,
        arguments: req.arguments,
    };
    let result = state.dispatcher.dispatch(call).await?;
    if let Some(err) = result.error {
        return Ok((StatusCode::BAD_REQUEST, Json(json!({ "error": err }))).into_response());
    }
    Ok((StatusCode::OK, Json(result.output)).into_response())
}

#[derive(Debug, Deserialize)]
struct BacktestRequest {
    ticker: String,
    start: String,
    end: String,
    interval: Option<String>,
    provider: Option<String>,
    params: Option<HashMap<String, String>>,
    initial_capital: Option<f64>,
    commission_rate: Option<f64>,
    periods_per_year: Option<u32>,
    risk_free_rate: Option<f64>,
}

async fn run_backtest<S: AuditStorage>(
    State(state): State<Arc<AppState<S>>>,
    Path(strategy): Path<String>,
    Json(req): Json<BacktestRequest>,
) -> Result<impl IntoResponse, AppError> {
    let range = parse_date_range(&req.start, &req.end)?;
    let interval = parse_interval(req.interval.as_deref())?;
    let ticker = Ticker::try_new(req.ticker)?;

    let series = state
        .market_data
        .fetch(&ticker, range, interval, req.provider.as_deref())
        .await?;
    let params = req.params.unwrap_or_default();
    let config = BacktestConfig {
        initial_capital: Decimal::from_f64(req.initial_capital.unwrap_or(100_000.0))
            .ok_or_else(|| QuantError::InvalidCommand("invalid initial_capital".into()))?,
        commission_rate: req.commission_rate.and_then(Decimal::from_f64),
        periods_per_year: req.periods_per_year.unwrap_or(252),
        risk_free_rate: req.risk_free_rate.unwrap_or(0.0),
    };

    let result = BacktestService::new().run_single_strategy(&strategy, &series, params, config)?;
    Ok(Json(backtest_to_json(&result)))
}

#[derive(Debug, Deserialize)]
struct PortfolioRequest {
    returns: HashMap<String, Vec<f64>>,
    risk_free_rate: Option<f64>,
    target_return: Option<f64>,
}

async fn optimize_portfolio<S: AuditStorage>(
    State(_state): State<Arc<AppState<S>>>,
    Json(req): Json<PortfolioRequest>,
) -> Result<impl IntoResponse, AppError> {
    let result = sqt_portfolio::PortfolioService::new().mean_variance(
        &req.returns,
        req.risk_free_rate.unwrap_or(0.0),
        req.target_return,
    )?;
    Ok(Json(serde_json::to_value(result).unwrap_or(Value::Null)))
}

async fn risk_parity<S: AuditStorage>(
    State(_state): State<Arc<AppState<S>>>,
    Json(req): Json<PortfolioRequest>,
) -> Result<impl IntoResponse, AppError> {
    let result = sqt_portfolio::PortfolioService::new().risk_parity(&req.returns)?;
    Ok(Json(serde_json::to_value(result).unwrap_or(Value::Null)))
}

#[derive(Debug, Deserialize)]
struct BlackLittermanRequest {
    returns: HashMap<String, Vec<f64>>,
    market_caps: HashMap<String, f64>,
    views: HashMap<String, f64>,
    tau: f64,
    risk_aversion: f64,
}

async fn black_litterman<S: AuditStorage>(
    State(_state): State<Arc<AppState<S>>>,
    Json(req): Json<BlackLittermanRequest>,
) -> Result<impl IntoResponse, AppError> {
    let result = sqt_portfolio::PortfolioService::new().black_litterman(
        &req.returns,
        &req.market_caps,
        &req.views,
        req.tau,
        req.risk_aversion,
    )?;
    Ok(Json(serde_json::to_value(result).unwrap_or(Value::Null)))
}

#[derive(Debug, Deserialize)]
struct ScreenRequest {
    filters: Vec<Value>,
    indicator_filters: Option<Vec<Value>>,
    start: Option<String>,
    end: Option<String>,
    interval: Option<String>,
    provider: Option<String>,
}

async fn screen<S: AuditStorage>(
    State(state): State<Arc<AppState<S>>>,
    Json(req): Json<ScreenRequest>,
) -> Result<impl IntoResponse, AppError> {
    let arguments = serde_json::json!({
        "filters": req.filters,
        "indicator_filters": req.indicator_filters,
        "start": req.start,
        "end": req.end,
        "interval": req.interval,
        "provider": req.provider,
    });
    let call = ToolCall {
        name: "screen_with_indicators".to_string(),
        arguments,
    };
    let result = state.dispatcher.dispatch(call).await?;
    if let Some(err) = result.error {
        return Ok((StatusCode::BAD_REQUEST, Json(json!({ "error": err }))).into_response());
    }
    Ok((StatusCode::OK, Json(result.output)).into_response())
}

async fn audit_verify<S: AuditStorage>(
    State(state): State<Arc<AppState<S>>>,
) -> Result<impl IntoResponse, AppError> {
    let verifier = AuditVerifier::new(state.audit_writer.storage());
    let result: sqt_audit::VerificationResult = verifier.verify().await?;
    Ok(Json(serde_json::to_value(result).unwrap_or(Value::Null)))
}

#[derive(Debug, Deserialize)]
struct CreateOrderRequest {
    ticker: String,
    side: String,
    order_type: String,
    quantity: f64,
    price: Option<f64>,
}

async fn create_order<S: AuditStorage>(
    State(state): State<Arc<AppState<S>>>,
    Json(req): Json<CreateOrderRequest>,
) -> Result<impl IntoResponse, AppError> {
    let side = parse_order_side(&req.side)?;
    let order_type = parse_order_type(&req.order_type)?;
    let quantity = Decimal::from_f64(req.quantity)
        .ok_or_else(|| QuantError::InvalidCommand("invalid quantity".into()))?;
    if quantity <= Decimal::ZERO {
        return Err(QuantError::InvalidCommand("quantity must be positive".into()).into());
    }
    let price = match req.price {
        Some(value) => Some(
            Decimal::from_f64(value)
                .ok_or_else(|| QuantError::InvalidCommand("invalid price".into()))?,
        ),
        None => None,
    };

    let service = OrderService::new(state.order_repo.clone());
    let order = service
        .create_order(req.ticker, side, order_type, quantity, price)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(order).map_err(|e| QuantError::Internal(e.into()))?),
    ))
}

async fn list_orders<S: AuditStorage>(
    State(state): State<Arc<AppState<S>>>,
) -> Result<impl IntoResponse, AppError> {
    let service = OrderService::new(state.order_repo.clone());
    let orders = service.list_orders().await?;
    Ok(Json(
        serde_json::to_value(orders).map_err(|e| QuantError::Internal(e.into()))?,
    ))
}

async fn get_order<S: AuditStorage>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let service = OrderService::new(state.order_repo.clone());
    let order = service
        .get_order(id)
        .await?
        .ok_or_else(|| QuantError::NotFound(format!("order {id} not found")))?;
    Ok(Json(
        serde_json::to_value(order).map_err(|e| QuantError::Internal(e.into()))?,
    ))
}

#[derive(Debug, Deserialize)]
struct TransitionRequest {
    action: String,
}

async fn transition_order<S: AuditStorage>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<TransitionRequest>,
) -> Result<impl IntoResponse, AppError> {
    let service = OrderService::new(state.order_repo.clone());
    let order = match req.action.as_str() {
        "submit" => service.submit_order(id).await?,
        "fill" => service.fill_order(id).await?,
        "cancel" => service.cancel_order(id).await?,
        _ => {
            return Err(QuantError::InvalidCommand(format!("unknown action {}", req.action)).into())
        }
    };
    Ok(Json(
        serde_json::to_value(order).map_err(|e| QuantError::Internal(e.into()))?,
    ))
}

async fn delete_order<S: AuditStorage>(
    State(state): State<Arc<AppState<S>>>,
    Path(id): Path<uuid::Uuid>,
) -> Result<StatusCode, AppError> {
    let service = OrderService::new(state.order_repo.clone());
    service
        .get_order(id)
        .await?
        .ok_or_else(|| QuantError::NotFound(format!("order {id} not found")))?;
    service.delete_order(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

use serde_json::json;
use sqt_core::Ohlcv;

fn parse_date_range(start: &str, end: &str) -> QuantResult<DateRange> {
    let start = chrono::NaiveDate::parse_from_str(start, "%Y-%m-%d")
        .map_err(|e| QuantError::InvalidCommand(format!("invalid start date: {e}")))?;
    let end = chrono::NaiveDate::parse_from_str(end, "%Y-%m-%d")
        .map_err(|e| QuantError::InvalidCommand(format!("invalid end date: {e}")))?;
    DateRange::try_new(start, end)
}

fn parse_interval(interval: Option<&str>) -> QuantResult<BarInterval> {
    match interval.unwrap_or("daily") {
        "daily" => Ok(BarInterval::Daily),
        "weekly" => Ok(BarInterval::Weekly),
        "monthly" => Ok(BarInterval::Monthly),
        other => Err(QuantError::InvalidCommand(format!(
            "invalid interval {other}"
        ))),
    }
}

fn parse_order_side(side: &str) -> QuantResult<OrderSide> {
    match side {
        "buy" => Ok(OrderSide::Buy),
        "sell" => Ok(OrderSide::Sell),
        _ => Err(QuantError::InvalidCommand(format!("invalid side {side}"))),
    }
}

fn parse_order_type(order_type: &str) -> QuantResult<OrderType> {
    match order_type {
        "market" => Ok(OrderType::Market),
        "limit" => Ok(OrderType::Limit),
        "stop" => Ok(OrderType::Stop),
        _ => Err(QuantError::InvalidCommand(format!(
            "invalid order_type {order_type}"
        ))),
    }
}

fn series_to_json(series: &[Ohlcv]) -> Value {
    json!(series
        .iter()
        .map(|b| json!({
            "date": b.date.to_string(),
            "open": b.open.to_f64(),
            "high": b.high.to_f64(),
            "low": b.low.to_f64(),
            "close": b.close.to_f64(),
            "volume": b.volume,
        }))
        .collect::<Vec<_>>())
}

fn indicator_to_json(result: &sqt_indicators::IndicatorResult) -> Value {
    let values = result
        .values
        .iter()
        .map(|(date, v)| json!({ "date": date.to_string(), "value": v.and_then(|d| d.to_f64()) }))
        .collect::<Vec<_>>();
    let extra: serde_json::Map<String, Value> = result
        .extra_series
        .iter()
        .map(|(name, series)| {
            let arr = series
                .iter()
                .map(|(date, v)| json!({ "date": date.to_string(), "value": v.and_then(|d| d.to_f64()) }))
                .collect::<Vec<_>>();
            (name.clone(), Value::Array(arr))
        })
        .collect();
    json!({
        "name": result.name,
        "params": result.params,
        "values": values,
        "extra_series": extra,
    })
}

fn metrics_to_json(result: &sqt_metrics::MetricsResult, metric: &str) -> QuantResult<Value> {
    let value = match metric {
        "sharpe" => result.sharpe,
        "sortino" => result.sortino,
        "max_drawdown" => result.max_drawdown,
        "var" => result.var,
        "cvar" => result.cvar,
        "beta" => result.beta,
        "alpha" => result.alpha,
        _ => {
            return Err(QuantError::InvalidCommand(format!(
                "unknown metric {metric}"
            )))
        }
    };
    Ok(json!({ "metric": metric, "value": value }))
}

fn backtest_to_json(result: &sqt_backtest::BacktestResult) -> Value {
    json!({
        "total_return": result.total_return,
        "max_drawdown": result.max_drawdown,
        "sharpe": result.sharpe,
        "number_of_trades": result.number_of_trades,
        "win_rate": result.win_rate,
    })
}

fn to_returns(series: &[Ohlcv]) -> QuantResult<Vec<f64>> {
    if series.len() < 2 {
        return Err(QuantError::DataQuality("need at least two bars".into()));
    }
    let mut sorted = series.to_vec();
    sorted.sort_by_key(|b| b.date);
    sorted
        .windows(2)
        .map(|w| {
            let prev = w[0]
                .close
                .to_f64()
                .ok_or_else(|| QuantError::DataQuality("invalid previous close".into()))?;
            let curr = w[1]
                .close
                .to_f64()
                .ok_or_else(|| QuantError::DataQuality("invalid current close".into()))?;
            if prev == 0.0 {
                return Err(QuantError::DataQuality("previous close is zero".into()));
            }
            Ok(curr / prev - 1.0)
        })
        .collect::<QuantResult<Vec<_>>>()
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

pub struct AppError(QuantError);

impl From<QuantError> for AppError {
    fn from(err: QuantError) -> Self {
        Self(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, body) = match &self.0 {
            QuantError::InvalidCommand(msg) => (StatusCode::BAD_REQUEST, json!({ "error": msg })),
            QuantError::NotFound(msg) => (StatusCode::NOT_FOUND, json!({ "error": msg })),
            QuantError::ProviderNotAvailable(msg) => {
                (StatusCode::SERVICE_UNAVAILABLE, json!({ "error": msg }))
            }
            QuantError::DataQuality(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, json!({ "error": msg }))
            }
            QuantError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": "internal error" }),
            ),
        };
        (status, Json(body)).into_response()
    }
}
