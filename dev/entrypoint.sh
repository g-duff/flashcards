#!/usr/bin/env bash
# PID 1 for the local dev image: start the backend on loopback (exactly as
# pi-flashcards-server.service does), then nginx. If either exits, `wait -n`
# returns and the container stops so the failure is visible. Signals are
# forwarded so `./dev/down.sh` stops the container promptly.
set -euo pipefail

export BIND_ADDR="127.0.0.1:8081"
export RUST_LOG="${RUST_LOG:-info}"

pi_flashcards_server &
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
