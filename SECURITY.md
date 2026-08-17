# Security Policy

## Supported Versions

Only the latest commit on `main` is actively supported with security updates.

## Reporting a Vulnerability

If you discover a security vulnerability, please email kevin@premialab.com with a clear description and reproduction steps. Do not open a public issue for security-sensitive bugs.

We will acknowledge receipt within 48 hours and aim to provide a fix or mitigation within 14 days.

## Security Practices

- Secrets and credentials are loaded from environment variables, never committed to source.
- API-key authentication is supported via `SQT_API_KEY` and enabled by default (`SQT_AUTH_ENABLED=true`).

> **Note:** API-key authentication is implemented for REST and gRPC and is enabled by default (`SQT_AUTH_ENABLED=true`). TLS termination and dependency scanning are not yet implemented. Container hardening is partial: `Dockerfile.native` uses a non-root distroless base, while the classic `Dockerfile` still runs as root; neither image has a `HEALTHCHECK`. Deploy behind a reverse proxy that provides TLS.
