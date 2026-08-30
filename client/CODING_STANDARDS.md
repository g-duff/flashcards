# Client coding standards

The `client/` uses a **pragmatic functional** TypeScript/React style —
not full FP-library adoption. Each decision below was made deliberately,
after weighing the tradeoff; they are conventions, not lint rules.

## TypeScript

- **Strict mode is mandatory.** `Optional<T>` / `Result<T, E>` are
  meaningless without `strictNullChecks`, so `tsconfig.app.json` sets
  `"strict": true`. Do not relax it.

- **`Result<T, E>` everywhere, not a hybrid.** Any function that can fail
  returns `Result<T, E>` — including the IO boundary. `fetch` and
  `JSON.parse` exceptions are caught inside `src/api.ts` and returned as
  a typed `ApiError`; nothing above that layer writes `try`/`catch`.
  Shape: `{ ok: true; value: T } | { ok: false; error: E }` — a boolean
  discriminant, not a `kind`/tag.

- **`undefined` only, no `null`.** The server does send `null` in JSON,
  but it is normalised to `undefined` at the API boundary (inside
  `api.ts`) so nothing above that layer ever sees `null`. Use
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

## React

- Arrow-function `const` components: `export const Foo = (props: Props) => …`.
- `type` over `interface` for props.
- Prefer values derived during render over `useEffect` + `useState`
  syncing. Reach for `useEffect` only for genuine outside-world effects
  (data fetch, subscriptions, timers).
