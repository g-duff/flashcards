// Effect types for the pragmatic-FP style this client follows. See
// CODING_STANDARDS.md for the reasoning. Bare types only — map / andThen
// / unwrapOr helpers get added here ad hoc, once a real call site needs
// chaining, not speculatively.

/** An absent value. `null` never appears above the API boundary — it is
 *  normalised to `undefined` inside `apiGet` / `apiSend`. */
export type Optional<T> = T | undefined;

/** Success or a typed failure. Boolean discriminant, not a tag. */
export type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };

export const ok = <T>(value: T): Result<T, never> => ({ ok: true, value });
export const err = <E>(error: E): Result<never, E> => ({ ok: false, error });
