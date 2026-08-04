//! Yahoo Finance market-data provider.
//!
//! Fetches historical OHLCV data from Yahoo's public CSV endpoint and maps the
//! response into [`Ohlcv`] bars.

use async_trait::async_trait;
use chrono::{NaiveDate, NaiveTime};
use rust_decimal::Decimal;
use sqt_core::{BarInterval, DateRange, Ohlcv, QuantError, Result, Ticker};
use std::time::Duration;
use tracing::{debug, instrument, warn};

use crate::port::MarketDataProvider;

const DEFAULT_BASE_URL: &str = "https://query1.finance.yahoo.com/v7/finance/download";

/// Yahoo Finance market-data provider.
///
/// The provider uses [`reqwest`] to download CSV data. The base URL can be
/// overridden (for example, to point to a WireMock server in tests) via
/// [`YahooFinanceProvider::with_base_url`].
#[derive(Debug, Clone)]
pub struct YahooFinanceProvider {
    client: reqwest::Client,
    base_url: String,
}

impl Default for YahooFinanceProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl YahooFinanceProvider {
    /// Creates a new provider using the production Yahoo Finance endpoint.
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL).expect("default base URL is valid")
    }

    /// Creates a new provider with a custom base URL.
    ///
    /// The full download URL is built as
    /// `{base_url}/{symbol}?period1={start}&period2={end}&interval={interval}&events=history`.
    ///
    /// # Errors
    ///
    /// Returns [`QuantError::InvalidCommand`] if `base_url` is empty or
    /// whitespace-only.
    pub fn with_base_url(base_url: impl Into<String>) -> Result<Self> {
        let base_url = base_url.into();
        if base_url.trim().is_empty() {
            return Err(QuantError::InvalidCommand(
                "Yahoo Finance base URL must be non-empty".to_string(),
            ));
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("valid reqwest client configuration");

        Ok(Self { client, base_url })
    }

    /// Maps a [`BarInterval`] to the string Yahoo expects in the query string.
    fn interval_str(interval: BarInterval) -> String {
        interval.to_string()
    }

    /// Converts a `NaiveDate` at midnight UTC to epoch seconds.
    fn naive_date_to_epoch(date: NaiveDate) -> i64 {
        date.and_time(NaiveTime::MIN).and_utc().timestamp()
    }
}

#[async_trait]
impl MarketDataProvider for YahooFinanceProvider {
    fn name(&self) -> &'static str {
        "yahoo_finance"
    }

    #[instrument(skip(self), fields(symbol = %ticker.symbol))]
    async fn fetch(
        &self,
        ticker: &Ticker,
        range: DateRange,
        interval: BarInterval,
    ) -> Result<Vec<Ohlcv>> {
        // Yahoo treats period2 as exclusive, so advance the end date by one day
        // to include it in the response.
        let end_exclusive = range.end.succ_opt().unwrap_or(range.end);
        let period1 = Self::naive_date_to_epoch(range.start);
        let period2 = Self::naive_date_to_epoch(end_exclusive);

        let url = format!(
            "{}/{symbol}?period1={period1}&period2={period2}&interval={interval}&events=history",
            self.base_url.trim_end_matches('/'),
            symbol = ticker.symbol,
            interval = Self::interval_str(interval),
        );

        debug!(%url, "fetching market data from Yahoo Finance");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                warn!(error = %e, "network error while contacting Yahoo Finance");
                QuantError::ProviderNotAvailable(format!("Yahoo Finance network error: {e}"))
            })?
            .error_for_status()
            .map_err(|e| {
                warn!(error = %e, "Yahoo Finance returned an error status");
                QuantError::ProviderNotAvailable(format!("Yahoo Finance HTTP error: {e}"))
            })?;

        let csv_text = response.text().await.map_err(|e| {
            warn!(error = %e, "failed to read Yahoo Finance response body");
            QuantError::ProviderNotAvailable(format!("Yahoo Finance response error: {e}"))
        })?;

        parse_yahoo_csv(ticker, range, &csv_text)
    }
}

/// Parses Yahoo CSV text into a vector of [`Ohlcv`] bars filtered by `range`.
fn parse_yahoo_csv(ticker: &Ticker, range: DateRange, csv_text: &str) -> Result<Vec<Ohlcv>> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(csv_text.as_bytes());
    let mut bars = Vec::new();

    for result in reader.records() {
        let record = result
            .map_err(|e| QuantError::DataQuality(format!("Yahoo Finance CSV parse error: {e}")))?;

        if record.len() < 7 {
            return Err(QuantError::DataQuality(
                "Yahoo Finance CSV row has too few columns".to_string(),
            ));
        }

        let date = NaiveDate::parse_from_str(&record[0], "%Y-%m-%d")
            .map_err(|e| QuantError::DataQuality(format!("Yahoo Finance date parse error: {e}")))?;

        // Filter to the requested inclusive date range.
        if date < range.start || date > range.end {
            continue;
        }

        let parse_decimal = |idx: usize, name: &str| -> Result<Decimal> {
            record[idx].trim().parse::<Decimal>().map_err(|e| {
                QuantError::DataQuality(format!("Yahoo Finance {name} parse error at {date}: {e}"))
            })
        };

        let open = parse_decimal(1, "Open")?;
        let high = parse_decimal(2, "High")?;
        let low = parse_decimal(3, "Low")?;
        let close = parse_decimal(4, "Close")?;
        let volume: i64 = record[6]
            .trim()
            .parse()
            .map_err(|e| QuantError::DataQuality(format!("volume parse error: {e}")))?;

        let bar =
            Ohlcv::try_new(ticker.clone(), date, open, high, low, close, volume).map_err(|e| {
                QuantError::DataQuality(format!("Yahoo Finance invalid OHLCV row at {date}: {e}"))
            })?;

        bars.push(bar);
    }

    Ok(bars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn sample_ticker_and_range() -> (Ticker, DateRange) {
        let ticker = Ticker::new("AAPL");
        let range = DateRange::try_new(
            NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
        )
        .unwrap();
        (ticker, range)
    }

    #[test]
    fn parse_yahoo_csv_filters_rows_and_maps_columns() {
        let (ticker, range) = sample_ticker_and_range();

        let csv = "Date,Open,High,Low,Close,Adj Close,Volume\n\
                   2024-01-01,150.00,155.00,149.00,154.00,154.00,1000\n\
                   2024-01-02,155.00,156.00,154.00,155.50,155.50,2000\n\
                   2024-01-03,156.00,157.00,155.00,156.50,156.50,3000\n\
                   2024-01-04,157.00,158.00,156.00,157.50,157.50,4000\n";

        let bars = parse_yahoo_csv(&ticker, range, csv).unwrap();
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].date, NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
        assert_eq!(bars[0].open, Decimal::from(155));
        assert_eq!(bars[1].close, Decimal::try_from(156.5).unwrap());
    }

    #[test]
    fn parse_yahoo_csv_returns_empty_for_empty_input() {
        let (ticker, range) = sample_ticker_and_range();
        let bars = parse_yahoo_csv(&ticker, range, "").unwrap();
        assert!(bars.is_empty());
    }

    #[test]
    fn parse_yahoo_csv_returns_data_quality_for_too_few_columns() {
        let (ticker, range) = sample_ticker_and_range();
        let csv = "Date,Open,High,Low,Close,Adj Close,Volume\n2024-01-02,155.00\n";
        let err = parse_yahoo_csv(&ticker, range, csv).unwrap_err();
        assert!(matches!(err, QuantError::DataQuality(_)));
        assert!(err.to_string().contains("too few columns"));
    }

    #[test]
    fn parse_yahoo_csv_returns_data_quality_for_bad_date() {
        let (ticker, range) = sample_ticker_and_range();
        let csv = "Date,Open,High,Low,Close,Adj Close,Volume\n\
                   not-a-date,155.00,156.00,154.00,155.50,155.50,2000\n";
        let err = parse_yahoo_csv(&ticker, range, csv).unwrap_err();
        assert!(matches!(err, QuantError::DataQuality(_)));
        assert!(err.to_string().contains("date parse error"));
    }

    #[test]
    fn parse_yahoo_csv_returns_data_quality_for_bad_decimal() {
        let (ticker, range) = sample_ticker_and_range();
        let csv = "Date,Open,High,Low,Close,Adj Close,Volume\n\
                   2024-01-02,not-a-number,156.00,154.00,155.50,155.50,2000\n";
        let err = parse_yahoo_csv(&ticker, range, csv).unwrap_err();
        assert!(matches!(err, QuantError::DataQuality(_)));
        assert!(err.to_string().contains("Open parse error"));
    }

    #[test]
    fn parse_yahoo_csv_returns_data_quality_for_bad_volume() {
        let (ticker, range) = sample_ticker_and_range();
        let csv = "Date,Open,High,Low,Close,Adj Close,Volume\n\
                   2024-01-02,155.00,156.00,154.00,155.50,155.50,not-a-volume\n";
        let err = parse_yahoo_csv(&ticker, range, csv).unwrap_err();
        assert!(matches!(err, QuantError::DataQuality(_)));
        assert!(err.to_string().contains("volume parse error"));
    }

    #[test]
    fn with_base_url_rejects_empty_string() {
        let err = YahooFinanceProvider::with_base_url("").unwrap_err();
        assert!(matches!(err, QuantError::InvalidCommand(_)));
        assert!(err.to_string().contains("base URL must be non-empty"));
    }
}
