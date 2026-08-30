# Running flashcards locally

One container, one command. nginx + the Rust backend in a single image, wired
the way the pi is: the backend binds `127.0.0.1:8081`, nginx is the only
published port, and nginx routes with the **real, unmodified `../nginx.conf`
fragment** loaded into a local copy of the fleet's shared nginx shell
(`sandy-bank.conf`).

```sh
./dev/up.sh                       # build + run   -> http://localhost:8080/flashcards/
./dev/down.sh                     # stop + remove
HTTP_PORT=9000 ./dev/up.sh        # publish elsewhere
podman logs -f sb-flashcards-dev  # both processes' logs
```

Only `podman` is needed on the host — the client bundle and the server binary are
both built inside the image.

## What it checks

The integration seam that `vite dev` can't: prefix stripping (`/flashcards/api/`
→ backend `/`), the SPA `alias` + `try_files` deep-link fallback, the exact-match
`/flashcards/openapi.yaml` route, and the `location /` fallthrough to the holding
page — all through the same fragment that ships to the pi.

| Verify | Expect |
|---|---|
| `curl -sf localhost:8080/flashcards/api/healthz` | `ok` |
| `curl -s localhost:8080/flashcards/api/cards` | `[{"id":1,...}, ...]` |
| `curl -s localhost:8080/flashcards/openapi.yaml \| head -1` | `openapi: 3.0.3` |
| `curl -s localhost:8080/flashcards/deck/42` | the SPA `index.html` (deep-link fallback) |
| `curl -s localhost:8080/` | `<h1>sandy-bank</h1>` (shell fallthrough) |
| `curl -si localhost:8081/` | connection refused — backend isn't published |

## Fidelity to the pi

**Matches:** Debian bookworm userland + nginx (the pi's OS and proxy), `BIND_ADDR`
loopback, `RUST_LOG`, nginx as the sole ingress, and the real routing fragment.

**Doesn't:** CPU arch and libc — this builds a native glibc release binary for the
build host, whereas production cross-compiles a static musl/ARMv6 binary (musl
there dodges an ARMv6-specific glibc bug that doesn't exist elsewhere). Also: no
systemd (an entrypoint script instead), and the backend runs as root in the
container rather than as `george`.

### Running the real ARMv6 artifact

The production `../server/Containerfile` already emits the exact
`arm-unknown-linux-musleabihf` binary. To run *that* under emulation:

```sh
( cd server && ./deploy.sh -b )        # -> server/dist/pi_flashcards_server
podman run --rm --platform linux/arm/v6 -e BIND_ADDR=0.0.0.0:8081 -p 8081:8081 \
  -v "$PWD/server/dist:/app:ro" docker.io/arm32v6/alpine /app/pi_flashcards_server
curl localhost:8081/healthz
```

One-time host setup: `podman run --rm --privileged docker.io/tonistiigi/binfmt
--install arm`. Slow to build and run — use it to confirm the real artifact boots
and serves, not for day-to-day work.

## The fast inner loop (unchanged)

For iterating on code, skip the container:

```sh
cd server && BIND_ADDR=127.0.0.1:8081 cargo run      # terminal 1
cd client && npm run dev                             # terminal 2 — HMR, proxies /flashcards/api
```

`dev/` is the prod-parity check you run before a deploy, not a replacement for
`npm run dev`.

## Porting to another app

Copy `dev/`, then change `APP` in `up.sh`/`down.sh`, `HTTP_PORT` if `8080` clashes,
and the `flashcards` / `pi_flashcards_server` literals in `Containerfile`.
