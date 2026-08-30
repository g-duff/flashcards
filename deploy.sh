#!/usr/bin/env bash
# Full deploy for the `flashcards` app. Three delegated concerns:
#   * server binary + systemd unit -> server/deploy.sh            (-b -c -r)
#   * client static bundle         -> client/deploy.sh            (-b -c)
#   * nginx routing fragment       -> deploy/install-fragment.sh  (-n)
#
# -b/-c/-r run in build -> copy -> restart order whatever order passed;
# -n is this layer's addition. No flags -> usage.
#
# Usage:
#   ./deploy.sh [pi-host] (-b|--build) (-c|--copy) (-r|--restart) (-n|--nginx)
#
# Examples:
#   ./deploy.sh -b -c -r -n              # full deploy: build, copy, restart, routing
#   ./deploy.sh -c -r 192.168.1.187      # redeploy already-built artifacts, no nginx
#   ./deploy.sh -n                       # just (re)install the routing fragment
#
# pi-host defaults to $PI_HOST, then pi-0.local.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NAME="flashcards"
PI_HOST="${PI_HOST:-pi-0.local}"

# Vendored helper (deploy/). Override with a Sandy Bank checkout by
# setting SANDY_BANK=/path/to/sandy-bank in the environment.
if [[ -n "${SANDY_BANK:-}" ]]; then
  INSTALL_FRAGMENT="$SANDY_BANK/components/nginx/install-fragment.sh"
else
  INSTALL_FRAGMENT="$SCRIPT_DIR/deploy/install-fragment.sh"
fi

usage() {
  cat <<'EOF'
Usage: ./deploy.sh [pi-host] (-b|--build) (-c|--copy) (-r|--restart) (-n|--nginx)

Phases (pass one or more; server/client phases run build -> copy -> restart):
  -b, --build      Cross-compile the binary + build the client bundle
  -c, --copy       Copy the built binary and client bundle onto the pi
  -r, --restart    Install the systemd unit and restart the server
  -n, --nginx      Install nginx.conf into /etc/nginx/sandy-bank.d/flashcards.conf
  -h, --help       Show this help

pi-host defaults to $PI_HOST, then pi-0.local.
EOF
}

server_flags=()
client_flags=()
do_nginx=false

for arg in "$@"; do
  case "$arg" in
    -b|--build)         server_flags+=(-b); client_flags+=(-b) ;;
    -c|--copy)          server_flags+=(-c); client_flags+=(-c) ;;
    -r|--restart|--run) server_flags+=(-r) ;;
    -n|--nginx)         do_nginx=true ;;
    -h|--help)          usage; exit 0 ;;
    -*)                 echo "Unknown option: $arg" >&2; usage >&2; exit 2 ;;
    *)                  PI_HOST="$arg" ;;
  esac
done

if [[ ${#server_flags[@]} -eq 0 && ${#client_flags[@]} -eq 0 && $do_nginx == false ]]; then
  usage >&2
  exit 1
fi

if [[ -d "$SCRIPT_DIR/server" && ${#server_flags[@]} -gt 0 ]]; then
  PI_HOST="$PI_HOST" "$SCRIPT_DIR/server/deploy.sh" "${server_flags[@]}" "$PI_HOST"
fi

if [[ -d "$SCRIPT_DIR/client" && ${#client_flags[@]} -gt 0 ]]; then
  PI_HOST="$PI_HOST" "$SCRIPT_DIR/client/deploy.sh" "${client_flags[@]}" "$PI_HOST"
fi

if $do_nginx; then
  PI_HOST="$PI_HOST" "$INSTALL_FRAGMENT" "$NAME" "$SCRIPT_DIR/nginx.conf" "$PI_HOST"
fi
