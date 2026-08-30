// The one seam to the backend. Every call returns a Result — network and
// JSON-parse exceptions are caught and turned into an `ApiError`, so no
// code above this layer writes try/catch (see CODING_STANDARDS.md).
//
// Paths are relative and carry the app prefix: nginx maps
// /flashcards/api/ to the binary and strips it. In dev, vite.config.ts
// proxies the same prefix to 127.0.0.1:8081.

import type { Optional, Result } from "./types/effects";
import { err, ok } from "./types/effects";

const API_BASE = "/flashcards/api";

export type ApiError =
  | { kind: "network"; detail: string }
  | { kind: "http"; status: number; message: string }
  | { kind: "malformed"; detail: string };

/** A vocabulary pair. The three text fields are immutable identity; only
 *  `notes` can be edited. `id` is a UUID derived from the texts. */
export type Term = {
  id: string;
  foreign_lang: string;
  foreign_text: string;
  pivot_text: string;
  notes: Optional<string>;
  created_at: string;
};

export type NewTerm = {
  foreign_lang: string;
  foreign_text: string;
  pivot_text: string;
  notes?: Optional<string>;
};

const request = async <T>(
  path: string,
  init?: RequestInit,
): Promise<Result<T, ApiError>> => {
  let response: Response;
  try {
    response = await fetch(`${API_BASE}${path}`, {
      headers: { "content-type": "application/json" },
      ...init,
    });
  } catch (cause) {
    return err({ kind: "network", detail: String(cause) });
  }

  const bodyText = await response.text();

  if (!response.ok) {
    const message = parseErrorMessage(bodyText) ?? response.statusText;
    return err({ kind: "http", status: response.status, message });
  }

  if (bodyText.length === 0) {
    return ok(undefined as T);
  }

  try {
    return ok(normaliseNulls(JSON.parse(bodyText)) as T);
  } catch (cause) {
    return err({ kind: "malformed", detail: String(cause) });
  }
};

export const listTerms = (): Promise<Result<Term[], ApiError>> =>
  request<Term[]>("/terms");

export const createTerm = (term: NewTerm): Promise<Result<Term, ApiError>> =>
  request<Term>("/terms", { method: "POST", body: JSON.stringify(term) });

export const patchTermNotes = (
  id: string,
  notes: Optional<string>,
): Promise<Result<Term, ApiError>> =>
  request<Term>(`/terms/${encodeURIComponent(id)}`, {
    method: "PATCH",
    // The server distinguishes "clear the notes" (null) from "absent";
    // Optional<string> maps to one or the other here.
    body: JSON.stringify({ notes: notes ?? null }),
  });

export const deleteTerm = (
  id: string,
): Promise<Result<{ deleted: string }, ApiError>> =>
  request<{ deleted: string }>(`/terms/${encodeURIComponent(id)}`, {
    method: "DELETE",
  });

// --- helpers ---------------------------------------------------------------

/** Server JSON can carry `null`; nothing above this layer should see it. */
const normaliseNulls = (value: unknown): unknown => {
  if (value === null) return undefined;
  if (Array.isArray(value)) return value.map(normaliseNulls);
  if (typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>).map(([k, v]) => [
        k,
        normaliseNulls(v),
      ]),
    );
  }
  return value;
};

/** The API's error convention is `{ "error": "<message>" }`. */
const parseErrorMessage = (bodyText: string): string | undefined => {
  try {
    const parsed: unknown = JSON.parse(bodyText);
    if (
      typeof parsed === "object" &&
      parsed !== null &&
      "error" in parsed &&
      typeof (parsed as { error: unknown }).error === "string"
    ) {
      return (parsed as { error: string }).error;
    }
  } catch {
    // fall through
  }
  return undefined;
};
