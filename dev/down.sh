#!/usr/bin/env bash
# Stop and remove the local dev container. The image is left in place for
# the next ./dev/up.sh; `podman rmi sb-flashcards-dev` to drop it too.
set -euo pipefail

APP="flashcards"
podman rm -f "sb-${APP}-dev"
