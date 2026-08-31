#!/usr/bin/env bash
# Stop and remove the local dev container. The image AND the SQLite data
# volume are left for the next ./dev/up.sh, so the deck persists across
# down + up. `podman rmi sb-<app>-dev` drops the image;
# `podman volume rm sb-<app>-dev-data` wipes the deck.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP="$(basename "$(dirname "$SCRIPT_DIR")")"
podman rm -f "sb-${APP}-dev"
