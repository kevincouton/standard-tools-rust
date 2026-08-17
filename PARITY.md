# Port Parity Matrix

This document compares the `standard-tools-rust` port against the other Standard-Tools language implementations. It is accurate as of the latest commit on `main`.

## Legend

- ✅ Implemented / available
- ⚠️ Partial, stub, or minimal implementation
- ❌ Not implemented
- N/A Not applicable for this transport/stack

## Transport & protocol support

| Feature | Rust | C# | Kotlin | Go | C++ |
|---|---|---|---|---|---|
| REST | ✅ | ✅ | ✅ | ✅ | ✅ |
| gRPC | ⚠️ health + agent | ❌ | ✅ | ⚠️ health only | ⚠️ health only |
| A2A | ⚠️ partial (get/cancel placeholders) | ❌ | ⚠️ tasks/send, no streaming | ⚠️ minimal | ⚠️ skeleton |
| MCP | ⚠️ HTTP-only | ❌ | ✅ SSE | ⚠️ HTTP-only | ⚠️ HTTP-only |
| SSE | ❌ | ❌ | ⚠️ MCP transport only | ❌ | ❌ |
| Docker / container image | ✅ | ❌ | ✅ | ✅ | ✅ |
| CLI | ⚠️ server + audit placeholders | ❌ | ⚠️ audit commands only | ✅ | ✅ |
| Container health checks | ❌ | ⚠️ HTTP only | ⚠️ actuator only | ✅ | ✅ |

## Domain modules

| Feature | Rust | C# | Kotlin | Go | C++ |
|---|---|---|---|---|---|
| Market data provider port | ✅ YF + Moka cache | ⚠️ interface / stub | ✅ YF, Polygon, Bloomberg stub | ✅ synthetic, YF, Polygon | ⚠️ synthetic only |
| Indicators | ✅ | ✅ | ✅ | ✅ | ✅ |
| Risk / return metrics | ✅ | ✅ | ✅ | ✅ | ✅ |
| Analysis (regression, cointegration, Hurst, PCA, correlation, options) | ⚠️ no multi-factor | ✅ library; ⚠️ only regression + options exposed | ✅ | ⚠️ no multi-factor | ⚠️ no multi-factor |
| Backtesting engine | ✅ | ✅ | ✅ | ✅ | ✅ |
| Walk-forward optimization | ✅ | ✅ | ✅ | ✅ | ✅ |
| Monte Carlo simulation | ✅ | ✅ | ✅ | ✅ | ✅ |
| Robustness / stress testing | ✅ | ❌ | ✅ | ❌ | ❌ |
| Portfolio mean-variance | ✅ | ✅ | ✅ | ✅ | ✅ |
| Portfolio risk parity | ✅ equal-risk-contribution | ⚠️ inverse-vol | ✅ equal-risk-contribution | ✅ equal-risk-contribution | ✅ equal-risk-contribution |
| Black-Litterman | ✅ | ✅ | ✅ | ✅ | ✅ |
| Screener | ⚠️ hardcoded provider | ⚠️ hardcoded provider | ⚠️ hardcoded provider | ⚠️ hardcoded provider | ⚠️ hardcoded provider |
| Hash-chained audit | ✅ | ✅ | ✅ | ✅ | ✅ |
| Agent tool dispatcher | ✅ (42 tools) | ✅ | ✅ | ✅ (19 tools) | ✅ (11 tools) |

## Security & audit

| Feature | Rust | C# | Kotlin | Go | C++ |
|---|---|---|---|---|---|
| API-key auth on REST | ✅ fail-closed | ✅ fail-closed | ✅ fail-closed | ✅ fail-closed | ✅ fail-closed |
| API-key auth on gRPC | ✅ | N/A | ✅ | ✅ | ❌ |
| TLS termination | ❌ | ❌ | ❌ | ❌ | ❌ |
| Audit provenance (git commit / version / seed) | ❌ none recorded | ⚠️ schema only | ⚠️ commit + version | ✅ all three | ✅ all three |
| Replay read-only / side-effect blocklist | ⚠️ blocklist, CLI placeholder | ❌ not implemented | ✅ blocklist | ❌ re-executes | ⚠️ read-only fetch, no re-execution |
| Persistent audit storage | ✅ PostgreSQL + memory | ❌ in-memory only | ✅ PostgreSQL | ✅ PostgreSQL + memory | ✅ PostgreSQL + memory |

## Operational hardening

| Feature | Rust | C# | Kotlin | Go | C++ |
|---|---|---|---|---|---|
| Request body limit | 16 MiB | 16 MiB | 16 MB + 4 MB gRPC | 16 MiB | 16 MiB |
| HTTP/gRPC request timeout | 60 s | configured | 30 s netty | configured | ❌ |
| Backtest bar cap | 50 000 | 50 000 | 50 000 | 50 000 | 50 000 |
| Monte Carlo simulation cap | 10 000 | 100 000 | 100 000 / 2 520 horizon | 100 000 | 100 000 |
| Walk-forward window cap | 10 000 | 10 000 | 10 000 | 10 000 | 10 000 |
| Walk-forward combination cap | 10 000 | 10 000 | 10 000 | 10 000 | 10 000 |
| Portfolio asset cap | 100 | 100 | 100 | 100 | 100 |
| Screener ticker cap | 100 | 500 | 500 | 500 | 500 |
| Structured logging / request tracing | ❌ | ❌ | ❌ | ❌ | ❌ |
| Metrics / Prometheus endpoint | ❌ | ❌ | ✅ | ❌ | ❌ |

## CI status

| Port | Status | Notes |
|---|---|---|
| Rust | ⚠️ config fixed | CI config was red for `cargo fmt` / `pipefail`; those causes are fixed locally. Pending act/upstream run for final confirmation |
| C# | ✅ green | `dotnet test` passes (88 tests) |
| Kotlin | ✅ green | unit / integration / e2e green; native build not validated locally |
| Go | ✅ green | `go test ./...` and image builds green locally |
| C++ | ❌ red | `rm -rf /var/lib/apt/lists/*` lacks permissions in GitHub Actions runner |

## Known limitations relevant to this port

- Audit records do not capture git commit, package version, or random seed.
- A2A `tasks/get` and `tasks/cancel` are placeholders.
- No container `HEALTHCHECK`; the classic image runs as root.

## Recommendations before a release tag

1. Add audit provenance fields (git commit, version, random seed).
2. Complete A2A get/cancel or remove the placeholder endpoints.
3. Add `HEALTHCHECK` and migrate the classic image to a non-root base.
