#!/usr/bin/env bash
# Runs the GitHub Actions workflow locally using nektos/act and Podman.
set -euo pipefail

ACT_BIN="${ACT_BIN:-act}"
CONTAINER_SOCKET="${CONTAINER_SOCKET:-$HOME/.local/share/containers/podman/machine/podman.sock}"

if ! command -v "$ACT_BIN" >/dev/null 2>&1; then
  echo "act not found. Install it with: mise install"
  exit 1
fi

echo "Running CI workflow locally with Podman..."
export DOCKER_HOST="unix://$CONTAINER_SOCKET"
"$ACT_BIN" -P ubuntu-latest=node:20-bookworm --container-daemon-socket "$CONTAINER_SOCKET" "$@"
