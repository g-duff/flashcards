#!/usr/bin/env bash
# PID 1 for the local dev image: start the backend on loopback (exactly as
# the systemd unit does on the pi), then nginx. If either exits, `wait -n`
# returns and the container stops so the failure is visible. Signals are
# forwarded so `./dev/down.sh` stops the container promptly.
set -euo pipefail

export BIND_ADDR="${BIND_ADDR:-127.0.0.1:8081}"   # up.sh passes the fragment's port
export RUST_LOG="${RUST_LOG:-info}"
# The SQLite file lives on a named volume (see dev/up.sh) so the deck
# survives ./dev/down.sh + ./dev/up.sh.
export DATABASE_PATH="${DATABASE_PATH:-/data/flashcards.db}"

app-server &
backend=$!
nginx -g 'daemon off;' &
nginx_pid=$!

shutdown() {
    kill -QUIT "$nginx_pid" 2>/dev/null || true   # nginx: graceful
    kill -TERM "$backend"   2>/dev/null || true   # backend: has a SIGTERM handler
}
trap shutdown TERM INT QUIT

wait -n
shutdown
wait
