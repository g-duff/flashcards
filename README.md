# flashcards

Language Practice Flashcards — a local, shared vocabulary-practice app. See
`CONTEXT.md` for the domain glossary and `.scratch/language-practice-flashcards/spec.md`
for the full specification.

## Running the stack

This project runs on plain [Podman](https://podman.io/) — no Docker
daemon required. See [`docs/running-the-stack.md`](docs/running-the-stack.md)
for the verified commands (works with or without a compose tool
installed).

- Client: http://localhost:3000
- Server: http://localhost:8080
- SQLite data persists in the `flashcards_sqlite_data` named volume
  across container restarts.

## Client (`client/`)

TypeScript/React app built with Vite.

```sh
npm install
npm run dev          # local dev server
npm run build         # type-check + production build
npm run lint           # oxlint
npm run format          # prettier --write
npm run format:check     # prettier --check
npm test                  # vitest
```

## Server (`server/`)

Rust/axum, backed by SQLite (raw SQL via `sqlx`), configured via
`config.yaml` (see `config.container.yaml` for the containerized variant).

```sh
cargo build            # compile
cargo test               # unit tests (functional core) + HTTP integration tests
cargo fmt                 # format
cargo clippy --all-targets  # lint
cargo run                    # start the server (reads ./config.yaml, or $CONFIG_PATH)
```

Configuration precedence: `$CONFIG_PATH` env var if set, else
`./config.yaml` relative to the working directory the server is started
from. YAML application defaults (including Learner Algorithm Settings)
apply to newly created Learners only; existing Learners keep their
persisted settings until they explicitly reset them.

Migrations are plain numbered `.sql` files under `migrations/` (path
configurable via `migrations_path`), applied automatically on startup.
