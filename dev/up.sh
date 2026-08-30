#!/usr/bin/env bash
# Build and run the whole app locally in one container (nginx + backend),
# wired like the pi. Idempotent — re-run after any change. podman is the
# only prerequisite; the client bundle and server binary build inside the
# image. See dev/README.md.
#
# Usage: ./dev/up.sh
#   HTTP_PORT=9000 ./dev/up.sh    # publish on a different host port
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"
APP="$(basename "$APP_DIR")"
IMAGE="sb-${APP}-dev"
HTTP_PORT="${HTTP_PORT:-8080}"

# The loopback port the fragment proxies to — read from the one place it
# is written, nginx.conf, so this never drifts. Falls back to 8081 for a
# client-only app (no proxy_pass line).
PORT="$(grep -oE '127\.0\.0\.1:[0-9]+' "$APP_DIR/nginx.conf" | head -1 | cut -d: -f2 || true)"
PORT="${PORT:-8081}"

echo "==> Building $IMAGE"
podman build -t "$IMAGE" --build-arg "APP=$APP" -f "$SCRIPT_DIR/Containerfile" "$APP_DIR"

echo "==> Running $IMAGE  ->  http://localhost:$HTTP_PORT/$APP/"
podman run -d --replace --name "$IMAGE" \
  -e "BIND_ADDR=127.0.0.1:$PORT" -p "$HTTP_PORT:80" "$IMAGE" >/dev/null

echo "    logs:  podman logs -f $IMAGE"
echo "    stop:  $SCRIPT_DIR/down.sh"
