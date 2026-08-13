# 01 — Project scaffolding: server, client, and Podman wiring

**What to build:** A running skeleton the rest of the app builds on. An operator can bring up the whole stack with one command and see the client talk to the server through the standard JSON envelope, with data surviving a restart.

**Blocked by:** None — can start immediately.

**Status:** done

- [ ] Rust/axum server boots, reading a YAML config file with (at least) database URL, migration path, session inactivity timeout, cookie lifetime, question-count bounds, supported-language list, and application-level algorithm defaults.
- [ ] SQLite is wired up via a configurable database URL; numbered/versioned migrations run on startup from a configurable migration path.
- [ ] Server exposes a health/status endpoint returning the project's standard success envelope (`status`, `data`, `meta.timestamp`).
- [ ] A shared error envelope helper exists (`status: "error"`, `error.code`, `error.message`, `error.details[]`) with optional fields always present and serialized as `null` when empty, ready for later endpoints to use.
- [ ] TypeScript/React client scaffold exists, buildable and runnable, with a layout/viewport suitable for iPhone Safari, and can successfully call the server health endpoint.
- [ ] Podman (via `compose.yaml` or plain `podman build`/`podman run` commands) runs client and server as separate services; SQLite is persisted through a shared container volume that survives a container restart.
- [ ] Existing npm scripts/repo conventions are used for client build/lint/format; server has an equivalent build/test convention documented.
