#!/usr/bin/env bash
# Installs one per-app nginx routing fragment into
# /etc/nginx/sandy-bank.d/ on the pi, validates the *whole* nginx config
# with `nginx -t`, and restores the previous fragment if validation
# fails — so a broken fragment can never take the shared proxy down.
#
# Vendored from the Sandy Bank repo (components/nginx/install-fragment.sh).
# No repo-relative dependencies: plain bash + ssh + scp, driven entirely
# by its arguments. Re-sync if the fleet ever changes the validate/
# rollback logic (rare).
#
# Usage: ./install-fragment.sh <name> <local-conf-path> [pi-host]
set -euo pipefail

NAME="${1:?fragment name required (arg 1)}"
SRC="${2:?local .conf path required (arg 2)}"
PI_HOST="${3:-${PI_HOST:-pi-0.local}}"
DEST="/etc/nginx/sandy-bank.d/${NAME}.conf"

[[ -f "$SRC" ]] || { echo "==> No such fragment file: $SRC" >&2; exit 1; }

echo "==> Installing '$NAME' fragment to $PI_HOST:$DEST"
scp "$SRC" "$PI_HOST:/tmp/${NAME}.conf.new"

# Swap + validate + rollback in one ssh session so a dropped connection
# can't leave a half-applied state.
ssh "$PI_HOST" "
  set -eu
  sudo mkdir -p /etc/nginx/sandy-bank.d
  if [ -f '$DEST' ]; then
    sudo cp -p '$DEST' '/tmp/${NAME}.conf.bak'; had_backup=1
  else
    had_backup=0
  fi
  sudo mv '/tmp/${NAME}.conf.new' '$DEST'

  if sudo nginx -t; then
    sudo systemctl reload nginx
    sudo rm -f '/tmp/${NAME}.conf.bak'
    echo '    ok — nginx reloaded'
  else
    echo '    nginx -t failed — rolling back '$NAME' fragment' >&2
    if [ \"\$had_backup\" -eq 1 ]; then
      sudo mv '/tmp/${NAME}.conf.bak' '$DEST'
    else
      sudo rm -f '$DEST'
    fi
    sudo nginx -t
    sudo systemctl reload nginx
    exit 1
  fi
"
echo "==> Done."
