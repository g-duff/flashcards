#!/usr/bin/env bash
# Build and run the whole flashcards app locally in one container (nginx +
# backend), wired like the pi. Idempotent — re-run after any change.
#
# Usage: ./dev/up.sh
#   HTTP_PORT=9000 ./dev/up.sh   # publish on a different host port
set -euo pipefail

APP="flashcards"
HTTP_PORT="${HTTP_PORT:-8080}"
IMAGE="sb-${APP}-dev"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "==> Building $IMAGE"
podman build -t "$IMAGE" -f "$SCRIPT_DIR/Containerfile" "$REPO_ROOT"

echo "==> Running $IMAGE on http://localhost:$HTTP_PORT/$APP/"
podman run -d --replace --name "$IMAGE" -p "$HTTP_PORT:80" "$IMAGE" >/dev/null

echo "    logs:  podman logs -f $IMAGE"
echo "    stop:  $SCRIPT_DIR/down.sh"
