# Client coding standards

The `client/` uses a **pragmatic functional** TypeScript/React style —
not full FP-library adoption. Each decision below was made deliberately,
after weighing the tradeoff; they are conventions, not lint rules.

The fleet-wide version of this document is
`sandy-bank/docs/frontend-coding-standards.md`; this file is the
flashcards-specific copy and adds nothing that contradicts it.

## TypeScript

- **Strict mode is mandatory.** `Optional<T>` / `Result<T, E>` are
  meaningless without `strictNullChecks`, so `tsconfig.app.json` sets
  `"strict": true`. Do not relax it.

- **`Result<T, E>` everywhere, not a hybrid.** Any function that can fail
  returns `Result<T, E>` — including the IO boundary. `fetch` and
  `JSON.parse` exceptions are caught inside `src/api/client.ts` and
  returned as a typed `ApiError`; nothing above that layer writes
  `try`/`catch`. Resource modules (`src/api/terms.ts`, …) sit alongside
  it and call its thin verb helpers (`apiGet` / `apiPost` / …).
  Shape: `{ ok: true; value: T } | { ok: false; error: E }` — a boolean
  discriminant, not a `kind`/tag.

- **`undefined` only, no `null`.** The server does send `null` in JSON,
  but it is normalised to `undefined` at the API boundary (inside
  `api/client.ts`) so nothing above that layer ever sees `null`. Use
  `Optional<T>` (= `T | undefined`) for an absent value. There is no
  `Nullable<T>`.

- **No Result helper library, no speculative helpers.** `src/types/effects.ts`
  holds the bare `Result` / `Optional` types and the `ok` / `err`
  constructors. `map` / `andThen` / `unwrapOr` get added there **only
  once a real call site needs chaining** — not up front. Pattern-match
  the `ok` boolean directly until then.

- **No lint-enforced immutability, no `pipe`/`compose` utility.**
  Convention only. Prefer `const`, spread over mutation, and pure helper
  functions, but the linter is not configured to require it.

## The `api/` layer

- **`src/api/` is the one seam to the backend.** `src/api/client.ts` is
  the only file that calls `fetch` — it owns the `/flashcards/api`
  prefix, the `ApiError` model, exception→`Result` conversion, the
  `null`→`undefined` normalisation, and the thin verb helpers (`apiGet`
  / `apiPost` / `apiPatch` / `apiDelete`).
- **One resource module per backend resource.** `src/api/terms.ts` holds
  the `Term` / `NewTerm` types and one-liner calls over the verb
  helpers — no `fetch`, no prefix, no error handling. The rest of the
  app imports `./api/terms` (and `./api/client` only for the `ApiError`
  type). Don't collapse this back into a single `src/api.ts`.

## Testing — the seam is `api`

Test doubles go in at `src/api/`, at exactly two levels:

- **Testing a resource module** (`src/api/terms.test.ts`): stub
  `globalThis.fetch` with `vi.spyOn` + `Response` objects. Assert the
  exact URL (prefix included), method, headers, and body that reach
  `fetch`, and that a given `Response` maps to the right `Result`
  (including `null`→`undefined` and each `ApiError` kind).
- **Testing anything above `api/`** (`App.test.tsx`, future
  hooks/components): `vi.mock("./api/terms", …)` and assert on the calls
  (`expect(api.createTerm).toHaveBeenCalledWith(…)`) and on how the
  component renders the returned `Result`. A component test must never
  touch `fetch`.

Pure view-state helpers (`upsertTerm`, `removeTerm`, `isCompleteDraft`,
`describeError`) are exported and unit-tested without rendering.

## React

- Arrow-function `const` components: `export const Foo = (props: Props) => …`.
- `type` over `interface` for props.
- Prefer values derived during render over `useEffect` + `useState`
  syncing. Reach for `useEffect` only for genuine outside-world effects
  (data fetch, subscriptions, timers).
