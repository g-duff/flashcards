import type { Optional, Result } from "../types/effects";
import type { ApiError } from "./client";
import { apiDelete, apiGet, apiPatch, apiPost } from "./client";

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

export const listTerms = (): Promise<Result<Term[], ApiError>> =>
  apiGet<Term[]>("/terms");

export const createTerm = (term: NewTerm): Promise<Result<Term, ApiError>> =>
  apiPost<Term>("/terms", term);

/** The outcome of a bulk import: how many Terms were newly written and
 *  how many were skipped because their id already existed. The two sum to
 *  the number of Terms sent. */
export type ImportReport = { imported: number; skipped: number };

/** Bulk-add Terms. The file is parsed to `NewTerm[]` in the browser (see
 *  `parseVocab`); the server only ever receives JSON. */
export const importTerms = (
  terms: NewTerm[],
): Promise<Result<ImportReport, ApiError>> =>
  apiPost<ImportReport>("/terms/import", terms);

export const patchTermNotes = (
  id: string,
  notes: Optional<string>,
): Promise<Result<Term, ApiError>> =>
  // The server distinguishes "clear the notes" (null) from "absent";
  // Optional<string> maps to one or the other here.
  apiPatch<Term>(`/terms/${encodeURIComponent(id)}`, { notes: notes ?? null });

export const deleteTerm = (
  id: string,
): Promise<Result<{ deleted: string }, ApiError>> =>
  apiDelete<{ deleted: string }>(`/terms/${encodeURIComponent(id)}`);
