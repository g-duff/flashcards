import type { ApiError } from "./api/client";

/** Turn an `ApiError` into a one-line message for the error UI. Exhaustive
 *  over the union so a new kind is a compile error, not a silent blank. */
export const describeError = (error: ApiError): string => {
  switch (error.kind) {
    case "network":
      return `Network error: ${error.detail}`;
    case "http":
      return `Server error ${error.status}: ${error.message}`;
    case "malformed":
      return `Bad response from server: ${error.detail}`;
  }
};
