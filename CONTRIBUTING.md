# Contributing

Thank you for your interest in improving Standard-Tools!

## Getting Started

1. Fork the repository and clone your fork.
2. Follow the README for build and test instructions for the specific language port.
3. Create a feature branch: `git checkout -b feature/my-change`.

## Pull Request Process

- Keep changes focused and minimal.
- Add or update tests for any new behavior or bug fix.
- Ensure the full test suite passes locally and in CI.
- Use conventional commit messages (e.g. `feat:`, `fix:`, `test:`, `docs:`).
- Update relevant documentation if public APIs or behavior change.

## Code Quality

- Follow the existing style and project structure.
- Prefer explicit, type-safe code and domain errors over generic exceptions.
- Add structured logging and observability for production paths.
- Do not commit secrets, credentials, or environment-specific configuration.

## Reporting Issues

Please open an issue with a clear description, reproduction steps, and expected behavior.
