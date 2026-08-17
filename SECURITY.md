# Security Policy

## Supported Versions

Only the latest commit on `main` is actively supported with security updates.

## Reporting a Vulnerability

If you discover a security vulnerability, please email kevin@premialab.com with a clear description and reproduction steps. Do not open a public issue for security-sensitive bugs.

We will acknowledge receipt within 48 hours and aim to provide a fix or mitigation within 14 days.

## Security Practices

- Secrets and credentials are loaded from environment variables, never committed to source.
- Production images run as non-root users.
- TLS and authentication are enabled by default in production deployments.
- Dependencies are kept up to date and scanned in CI.
