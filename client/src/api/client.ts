// The one seam to the backend. Every call returns a Result — network and
// JSON-parse exceptions are caught and turned into an `ApiError`, so no
// code above this layer writes try/catch (see CODING_STANDARDS.md).
//
// Paths are relative and carry the app prefix: nginx maps
// /flashcards/api/ to the binary and strips it. In dev, vite.config.ts
// proxies the same prefix to 127.0.0.1:8081.
//
// Resource modules (terms.ts, …) sit alongside this file and call the
// thin verb helpers below; they never touch `fetch` directly.

import type { Result } from "../types/effects";
import { err, ok } from "../types/effects";

const API_BASE = "/flashcards/api";

export type ApiError =
  | { kind: "network"; detail: string }
  | { kind: "http"; status: number; message: string }
  | { kind: "malformed"; detail: string };

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

export const apiGet = <T>(path: string): Promise<Result<T, ApiError>> =>
  request<T>(path);

export const apiPost = <T>(
  path: string,
  body: unknown,
): Promise<Result<T, ApiError>> =>
  request<T>(path, { method: "POST", body: JSON.stringify(body) });

export const apiPatch = <T>(
  path: string,
  body: unknown,
): Promise<Result<T, ApiError>> =>
  request<T>(path, { method: "PATCH", body: JSON.stringify(body) });

export const apiDelete = <T>(path: string): Promise<Result<T, ApiError>> =>
  request<T>(path, { method: "DELETE" });

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
