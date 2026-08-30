#!/usr/bin/env bash
# Stop and remove the local dev container. The image is left for the next
# ./dev/up.sh; `podman rmi sb-<app>-dev` to drop it too.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP="$(basename "$(dirname "$SCRIPT_DIR")")"
podman rm -f "sb-${APP}-dev"
