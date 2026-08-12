# Running the stack with Podman

This project builds and runs with plain [Podman](https://podman.io/) — no
Docker daemon required. Container build files are named `Containerfile`
(Podman's neutral name for what Docker calls a `Dockerfile`; Podman reads
either name, but this repo standardizes on `Containerfile`).

There are two ways to run the stack, depending on what's installed.

## Option A: `podman compose` / `podman-compose` (if available)

If your machine has a compose implementation wired up to Podman (the
`podman compose` plugin, or the standalone `podman-compose` tool), the
root `compose.yaml` works as-is:

```sh
podman compose up --build
```

Check first with `podman compose version` or `which podman-compose` — a
bare Podman install does not include either by default.

## Option B: plain `podman` commands (no compose tool needed)

This is the path verified against this repo. It reproduces what
`compose.yaml` describes — two services on a shared network, one named
volume for the SQLite database — using only `podman build` and
`podman run`.

```sh
# 1. Build both images (reads server/Containerfile and client/Containerfile)
podman build -t flashcards-server ./server
podman build -t flashcards-client ./client

# 2. Create the network and the persistent volume once
podman network create flashcards-net
podman volume create flashcards_sqlite_data

# 3. Run the server. Name it "server" — the client's nginx config
#    (client/nginx.conf) proxies /api/ to http://server:8080, resolving
#    that hostname via Podman's network DNS.
podman run -d --name server --network flashcards-net \
  -v flashcards_sqlite_data:/data -p 8080:8080 localhost/flashcards-server

# 4. Run the client
podman run -d --name client --network flashcards-net \
  -p 3000:80 localhost/flashcards-client
```

Then:

- Client: http://localhost:3000
- Server: http://localhost:8080
- The client also proxies `/api/*` through to the server, so
  `http://localhost:3000/api/health` works too.

### Verifying persistence

The SQLite database lives on the `flashcards_sqlite_data` named volume,
not inside the server container, so recreating the container preserves
data — the equivalent of `docker-compose down && docker-compose up`:

```sh
podman rm -f server
podman run -d --name server --network flashcards-net \
  -v flashcards_sqlite_data:/data -p 8080:8080 localhost/flashcards-server

podman exec server ls -la /data   # flashcards.db is still there
curl -s http://localhost:8080/api/health
```

### Cleaning up

```sh
podman rm -f server client
podman network rm flashcards-net
podman volume rm flashcards_sqlite_data   # destroys practice data
```

## Configuration

- `server/config.yaml` — local (non-containerized) development config.
- `server/config.container.yaml` — baked into the server image at
  `/app/config.yaml` and used by default in both options above (selected
  via the `CONFIG_PATH` environment variable set in `server/Containerfile`).
- `client` reads `VITE_API_BASE_URL` at *build* time (see
  `client/Containerfile`'s `ARG`/`ENV`). Both options above build with it
  unset, so the client calls the server through nginx's relative `/api/`
  proxy rather than an absolute URL.
