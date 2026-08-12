# Client Coding Standards

Pragmatic functional TypeScript for a React + Vite app. "Pragmatic" means: prefer
immutability, pure functions, and explicit effect types where they earn their keep —
but don't reach for a dedicated FP library or build abstractions before a call site
needs them. This document is the authority for both human contributors and coding
agents working in `client/`.

## Type checking

`tsconfig.app.json` has `"strict": true`. This is load-bearing, not optional: every
rule below about `Optional<T>` and `Result<T, E>` only holds because
`strictNullChecks` forces callers to handle the "missing" branch. Do not disable
strict mode to make a change compile faster.

## Effect types

Shared effect types live in `src/types/effects.ts`.

```ts
export type Optional<T> = T | undefined

export type Result<T, E> = { ok: true; value: T } | { ok: false; error: E }
```

### `Optional<T>` — the only "absence" type

Use `Optional<T>` (i.e. `T | undefined`) for any value that may be missing. Do not
use `null` in application code — `undefined` is the sole representation of "no
value".

`null` still appears at the edges of the system (JSON responses, some DOM/browser
APIs). Normalize it to `undefined` as soon as the value crosses into application
code — see "API boundary" below. Once normalized, `null` should not appear again.

```ts
// Good
function findCard(id: string): Optional<Card> {
  return cards.find((card) => card.id === id)
}

// Avoid
function findCard(id: string): Card | null { ... }
```

### `Result<T, E>` — the only error-handling type

Prefer `Result<T, E>` over throwing for **any** function that can fail, including
network/IO failures. A function's signature should tell the caller a failure is
possible; a caller should not need to know to wrap a call in `try/catch` to avoid a
crash.

```ts
async function apiGet<T>(path: string): Promise<Result<T, ApiError>> {
  try {
    const response = await fetch(path)
    const body = await response.json()
    if (body.status === 'error') {
      return { ok: false, error: new ApiError(body.error.message, body) }
    }
    return { ok: true, value: body.data }
  } catch (cause) {
    return { ok: false, error: new ApiError(toMessage(cause)) }
  }
}
```

Narrow with the `ok` discriminant, not a truthiness check:

```ts
const result = await apiGet<HealthData>('/api/health')
if (result.ok) {
  console.log(result.value.status)
} else {
  console.error(result.error.message)
}
```

**Exception:** genuine programmer errors (invariant violations, "this should be
impossible") may still throw. `Result` is for failures a caller is expected to
handle as normal control flow, not for bugs.

**No helper library yet.** Don't import `fp-ts`, `effect`, or similar, and don't
pre-build a set of `Result` combinators (`map`, `andThen`, `unwrapOr`, ...) before a
call site actually needs one. When a real call site would clearly benefit from
chaining instead of manual `if (result.ok)` narrowing, add the specific helper it
needs, colocated in `src/types/effects.ts`, e.g.:

```ts
function mapResult<T, U, E>(result: Result<T, E>, fn: (value: T) => U): Result<U, E> {
  return result.ok ? { ok: true, value: fn(result.value) } : result
}
```

Add helpers on demand, not speculatively — a `Result` with two or three
narrow-and-branch call sites is simpler to read than one hidden behind combinators
nobody else on the team has learned yet.

### API boundary: normalizing `null`

Server responses may contain `null` for absent fields (see `SuccessEnvelope<T>` in
`src/api/client.ts`). Convert `null` to `undefined` at the point where the response
body is unwrapped into a typed value — inside `apiGet`, not in every caller:

```ts
return { ok: true, value: (body.data ?? undefined) as T }
```

Code above the API layer should never need to check for `null`.

## Immutability and purity

Convention, not lint-enforced (no dedicated `no-let`/`no-param-reassign` rules are
configured — keep it that way unless a real problem shows up):

- Prefer `const` over `let`. Reach for `let` only when a loop or accumulator
  genuinely needs to be reassigned.
- Don't mutate function parameters or objects/arrays you don't own. Build a new
  value and return it instead.
- Favor pure functions (same input → same output, no side effects) for anything
  that isn't directly performing I/O or updating React state. Push side effects
  (`fetch`, `console`, state setters) to the edges — event handlers, `useEffect`,
  API modules — not into shared helper functions.

## Composition

Use native array methods (`.map`, `.filter`, `.reduce`) and optional
chaining/nullish coalescing (`?.`, `??`) to express transformations. Do not
introduce a `pipe`/`compose` utility — plain method chaining is enough at this
project's size and doesn't require learning a new call convention.

## React conventions

- Components are `const` arrow functions: `const Foo = (props: FooProps) => { ... }`.
- Type component props with `type`, not `interface` — this keeps props consistent
  with `Optional`/`Result` and other type aliases used throughout the codebase.
- Prefer values derived directly during render over syncing state with
  `useEffect` + `useState`. Only use `useEffect` for actual side effects (fetching
  data, subscriptions) — not to keep one piece of state in sync with another that
  could just be computed.
- Discriminated unions are the standard way to model UI state with multiple
  distinct shapes (loading/success/error, etc.), e.g. `ServerStatus` in `App.tsx`.
  This is a UI-state concern, separate from `Result<T, E>` (a function's return
  value) — don't force the two together.

## Formatting

Formatting is handled by Prettier (`npm run format`) and linted by oxlint
(`npm run lint`); their configs are the source of truth for things like semicolons,
quote style, and line width. This document does not restate them.
