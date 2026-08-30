#!/usr/bin/env bash
# Cross-compiles this crate for the Pi Zero W via `podman build` (which
# runs the whole build in the container and extracts just the binary —
# see Containerfile's `FROM scratch` stage), copies it to the pi, and
# installs/restarts the systemd unit.
#
# Usage: ./deploy.sh [pi-host] (-b|--build) (-c|--copy) (-r|--restart)
#   -b  Cross-compile into dist/
#   -c  Copy the built binary onto the pi (needs a prior -b)
#   -r  Install/update pi-flashcards-server.service and restart it —
#       systemd, not this script, is what daemonizes and supervises it.
# pi-host defaults to $PI_HOST, then pi-0.local.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST_DIR="$SCRIPT_DIR/dist"
BIN_NAME="$(grep '^name' "$SCRIPT_DIR/Cargo.toml" | head -1 | sed -E 's/name *= *"(.*)"/\1/')"
BIN_PATH="$DIST_DIR/$BIN_NAME"
SERVICE_FILE="pi-flashcards-server.service"

PI_HOST="${PI_HOST:-pi-0.local}"
DO_BUILD=false DO_COPY=false DO_RESTART=false

for arg in "$@"; do
  case "$arg" in
    -b|--build)         DO_BUILD=true ;;
    -c|--copy)          DO_COPY=true ;;
    -r|--restart|--run) DO_RESTART=true ;;
    -h|--help)          sed -n '2,13p' "$0"; exit 0 ;;
    -*) echo "Unknown option: $arg" >&2; exit 2 ;;
    *)  PI_HOST="$arg" ;;
  esac
done
$DO_BUILD || $DO_COPY || $DO_RESTART || { sed -n '2,13p' "$0"; exit 1; }

if $DO_BUILD; then
  echo "==> Building $BIN_NAME (arm-unknown-linux-musleabihf)"
  podman build -o "type=local,dest=$DIST_DIR" -f "$SCRIPT_DIR/Containerfile" "$SCRIPT_DIR"
  file "$BIN_PATH" || true
fi

if $DO_COPY; then
  [[ -f "$BIN_PATH" ]] || { echo "==> No binary at $BIN_PATH — run -b first" >&2; exit 1; }
  echo "==> Copying to $PI_HOST:~/$BIN_NAME"
  # scp to a temp name + mv into place: once systemd owns it the
  # destination is a running executable and scp can't open it for
  # writing. A rename works while the old inode is still executing.
  scp "$BIN_PATH" "$PI_HOST:~/$BIN_NAME.new"
  ssh "$PI_HOST" "chmod +x ~/$BIN_NAME.new && mv ~/$BIN_NAME.new ~/$BIN_NAME"
fi

if $DO_RESTART; then
  echo "==> Installing/updating $SERVICE_FILE on $PI_HOST"
  scp "$SCRIPT_DIR/$SERVICE_FILE" "$PI_HOST:/tmp/$SERVICE_FILE"
  ssh "$PI_HOST" "
    sudo mv /tmp/$SERVICE_FILE /etc/systemd/system/$SERVICE_FILE
    sudo systemctl daemon-reload
    sudo systemctl enable ${SERVICE_FILE%.service}
    sudo systemctl restart ${SERVICE_FILE%.service}
    sudo systemctl status ${SERVICE_FILE%.service} --no-pager -n 10
  "
fi
