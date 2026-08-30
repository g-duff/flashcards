#!/usr/bin/env bash
# Builds the client on this machine (Vite) and copies the static bundle
# to the pi, where nginx serves it at /flashcards/. Build-on-dev-box,
# copy-to-pi — same model as server/deploy.sh, minus a restart.
#
# Usage: ./deploy.sh [pi-host] (-b|--build) (-c|--copy)
set -euo pipefail

APP_NAME="flashcards"            # web root is /var/www/sandy-bank/$APP_NAME
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WEB_ROOT="/var/www/sandy-bank/$APP_NAME"
STAGING="/tmp/sandy-bank-$APP_NAME-client"
PI_HOST="${PI_HOST:-pi-0.local}"
DO_BUILD=false DO_COPY=false

for arg in "$@"; do
  case "$arg" in
    -b|--build) DO_BUILD=true ;;
    -c|--copy)  DO_COPY=true ;;
    -h|--help)  sed -n '2,7p' "$0"; exit 0 ;;
    -*) echo "Unknown option: $arg" >&2; exit 2 ;;
    *)  PI_HOST="$arg" ;;
  esac
done
$DO_BUILD || $DO_COPY || { echo "pass -b and/or -c" >&2; exit 1; }

if $DO_BUILD; then
  echo "==> Building client (npm ci && npm run build)"
  ( cd "$SCRIPT_DIR" && npm ci && npm run build )      # -> dist/
fi

if $DO_COPY; then
  [[ -f "$SCRIPT_DIR/dist/index.html" ]] || { echo "no dist/ — run -b first" >&2; exit 1; }
  echo "==> Copying dist/ to $PI_HOST:$WEB_ROOT"
  # rm -rf + cp, not rsync: rsync isn't guaranteed on the pi and the
  # target is a disposable static bundle. Only $WEB_ROOT is touched.
  ssh "$PI_HOST" "rm -rf '$STAGING'"
  scp -r "$SCRIPT_DIR/dist" "$PI_HOST:$STAGING"
  ssh "$PI_HOST" "
    sudo rm -rf '$WEB_ROOT'
    sudo mkdir -p '$WEB_ROOT'
    sudo cp -r '$STAGING'/. '$WEB_ROOT/'
    sudo chown -R root:root '$WEB_ROOT'
    sudo chmod -R a+rX '$WEB_ROOT'
    rm -rf '$STAGING'
  "
fi
