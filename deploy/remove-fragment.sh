#!/usr/bin/env bash
# Removes one per-app nginx routing fragment from
# /etc/nginx/sandy-bank.d/ on the pi and reloads. Use when
# decommissioning an app.
#
# Vendored from the Sandy Bank repo (components/nginx/remove-fragment.sh).
#
# Usage: ./remove-fragment.sh <name> [pi-host]
set -euo pipefail

NAME="${1:?fragment name required (arg 1)}"
PI_HOST="${2:-${PI_HOST:-pi-0.local}}"
DEST="/etc/nginx/sandy-bank.d/${NAME}.conf"

echo "==> Removing '$NAME' fragment from $PI_HOST:$DEST"
ssh "$PI_HOST" "
  set -eu
  if [ ! -f '$DEST' ]; then echo '    not installed — nothing to do'; exit 0; fi
  sudo rm -f '$DEST'
  sudo nginx -t
  sudo systemctl reload nginx
  echo '    ok — nginx reloaded'
"
echo "==> Done."
