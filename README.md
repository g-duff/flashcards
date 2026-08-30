# flashcards

A tiny flashcards app for the [Sandy Bank](../sandy-bank) homelab fleet.
List cards, add a card, review front/back. The deck lives in memory in
the backend — a redeploy resets it to a small seed; swapping in a
SQLite-backed store is the intended next step.

This repo is **external** to the Sandy Bank monorepo: it has its own git
history but plugs into every seam the fleet provides (front end, back
end, build, API docs, nginx routing), following
`sandy-bank/docs/SCAFFOLD-EXTERNAL.md`.

## Parts

| Path | What it is |
|------|------------|
| `server/` | Rust / axum backend — a static musl/ARMv6 binary bound to `127.0.0.1:8081`. In-memory card store, OpenAPI spec compiled in. |
| `client/` | Vite + React + TypeScript SPA, built to `/var/www/sandy-bank/flashcards/`. See `client/CODING_STANDARDS.md`. |
| `nginx.conf` | Routing fragment — bare `location` blocks, installed to `/etc/nginx/sandy-bank.d/flashcards.conf`. |
| `deploy.sh` | One-command deploy: forwards phases to `server/` and `client/`, installs the nginx fragment. |
| `deploy/` | Vendored `install-fragment.sh` / `remove-fragment.sh` (from `sandy-bank/components/nginx/`). |

## Prefix and port

| | Value | Registered fleet-side |
|---|---|---|
| Path prefix | `/flashcards/` | `sandy-bank/apps/README.md` routing model + port registry |
| Backend loopback bind | `127.0.0.1:8081` | `sandy-bank/apps/README.md` port registry (row: `flashcards` / `/flashcards/` / `:8081`) |

The API is mounted at `/flashcards/api/` (nginx strips the prefix before
proxying); the SPA is served at `/flashcards/`; the OpenAPI spec is at
`/flashcards/openapi.yaml`, served by the binary.

## Dependencies on the fleet

- The **shared nginx shell** must be deployed on the pi
  (`/etc/nginx/sandy-bank.d/` exists and is `include`d by an enabled
  server block on port 80). See `sandy-bank/components/nginx/` or
  `SCAFFOLD-EXTERNAL.md` Appendix A. Nothing this app does requires
  editing the shared `sandy-bank.conf` server block.
- **Shared `/docs/`**: this app serves an API, so add one line to
  `sandy-bank/components/swagger-ui/swagger-initializer.js` —
  `{ name: "flashcards", url: "/flashcards/openapi.yaml" }` (already
  present, commented out) — and redeploy that component. Until then the
  spec is still live at `http://<pi>/flashcards/openapi.yaml` for any
  OpenAPI viewer.

## Deploy

```sh
# Full deploy: build + copy binary and client, restart the service, install routing
PI_HOST=192.168.1.187 ./deploy.sh -b -c -r -n

# Redeploy already-built artifacts, no nginx change
PI_HOST=192.168.1.187 ./deploy.sh -c -r

# Just (re)install the routing fragment
PI_HOST=192.168.1.187 ./deploy.sh -n
```

Prerequisites on the dev box: `podman` (Rust cross-build runs in a
container), `node`/`npm`, `ssh`/`scp` access to the pi.

## Verify

```sh
curl http://pi-0.local/flashcards/api/healthz     # -> ok
curl http://pi-0.local/flashcards/api/cards       # -> [ {id,front,back}, ... ]
curl http://pi-0.local/flashcards/                 # -> the SPA index.html
curl http://pi-0.local/flashcards/openapi.yaml     # -> the spec
curl http://<pi-lan-ip>:8081/healthz              # connection refused (loopback-only)
open  http://pi-0.local/docs/                      # flashcards in the dropdown (once the line is uncommented)
```

## Run locally

Whole app in one container (nginx + backend), wired like the pi and using the
real `nginx.conf` fragment. Needs only `podman`:

```sh
./dev/up.sh        # build + run  -> http://localhost:8080/flashcards/
./dev/down.sh      # stop + remove
```

See `dev/README.md` for what it verifies, pi-fidelity notes, and the real-ARMv6
option.

## Local development (fast inner loop)

```sh
# backend
cd server && BIND_ADDR=127.0.0.1:8081 cargo run

# frontend (proxies /flashcards/api to 127.0.0.1:8081)
cd client && npm install && npm run dev
```

## Removing the app

```sh
PI_HOST=192.168.1.187 deploy/remove-fragment.sh flashcards
```

Then on the pi:

```sh
sudo systemctl disable --now pi-flashcards-server
sudo rm /etc/systemd/system/pi-flashcards-server.service
sudo rm -rf /var/www/sandy-bank/flashcards
sudo rm -f ~/pi_flashcards_server
```

And undo the coordination points: free `:8081` in the fleet port
registry, and remove/re-comment the `flashcards` line in
`sandy-bank/components/swagger-ui/swagger-initializer.js` (redeploy that
component).
