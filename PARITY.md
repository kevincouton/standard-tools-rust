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
| Analysis (regression, cointegration, Hurst, PCA, correlation, options) | ✅ | ✅ library; ⚠️ only regression + options exposed | ✅ | ⚠️ no multi-factor | ⚠️ no multi-factor |
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
| Persistent audit storage | ✅ PostgreSQL + memory | ✅ SQLite + memory | ✅ PostgreSQL | ✅ PostgreSQL + memory | ✅ PostgreSQL + memory |

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

Validation below was performed locally with `nektos/act` on `linux/arm64` (Podman) using the workflow job(s) that exercise the core build and tests.

| Port | Status | Notes |
|---|---|---|
| Rust | ✅ green | `act push --job test` passes; artifact upload skipped under `env.ACT` |
| C# | ✅ green | `act push --job build-and-test` passes |
| Kotlin | ✅ green | `act push --job unit-tests` passes; native build not validated locally |
| Go | ✅ green | `act push --job quality` passes |
| C++ | ✅ green | `act push --job quality` passes (build + ctest)

## Known limitations relevant to this port

- Audit records do not capture git commit, package version, or random seed.
- A2A `tasks/get` and `tasks/cancel` are placeholders.
- No container `HEALTHCHECK`; the classic image runs as root.

## Outstanding P0/P1 gaps (deferred)

The following items were identified in the staff-engine audit and are explicitly documented rather than hidden behind false claims:

1. **TLS termination** — not implemented in any port. Deploy behind a reverse proxy that terminates TLS.
2. **Structured logging / request tracing** — `tracing` is used internally but request IDs are not propagated to HTTP/gRPC responses and logs are not uniformly structured.
3. **Full A2A/MCP semantics** — A2A `tasks/get` and `tasks/cancel` are placeholders; MCP is HTTP-only and lacks full protocol compliance.
4. **Audit provenance** — audit records do not capture git commit, package version, or per-request random seed.
5. **Container hardening** — the classic `Dockerfile` runs as root and has no `HEALTHCHECK`.
6. **Dependency scanning** — add `cargo-deny` / `cargo-audit` or Dependabot to CI.

## Recommendations before a release tag

1. Add audit provenance fields (git commit, version, random seed).
2. Complete A2A get/cancel or remove the placeholder endpoints.
3. Add `HEALTHCHECK` and migrate the classic image to a non-root base.
