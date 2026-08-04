# standard-tools-rust

Rust port of the Standard-Tools quantitative finance toolkit.

## Stack

- Rust 1.82+
- Axum (REST)
- Tonic (gRPC)
- SQLx (PostgreSQL)
- Moka (cache)
- Ndarray / Nalgebra / Statrs (math)

## Quick Start

```bash
mise install
mise run build
mise run test
```

## Crates

| Crate | Purpose |
|-------|---------|
| sqt-core | Shared errors and value objects |
| sqt-marketdata | Market data providers and cache |
| sqt-indicators | Technical indicators |
| sqt-metrics | Risk and return metrics |
| sqt-analysis | Regression, cointegration, Hurst, PCA, options |
| sqt-backtest | Strategy backtesting engines |
| sqt-portfolio | Portfolio optimization |
| sqt-screener | Fundamental screener |
| sqt-agent | 42+ tool registry and dispatcher |
| sqt-audit | Hash-chained audit records |
| sqt-orders | Order domain and persistence |
| sqt-api | REST, gRPC, A2A, MCP, CLI |
