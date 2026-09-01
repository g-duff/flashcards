// Browser-side parsing for the bulk-import control. The learner picks a
// delimited text file and a delimiter; this turns it into `NewTerm`s and
// a list of the lines it could not read. The server only ever receives
// JSON — all file and delimiter handling stops here.

import type { NewTerm } from "./api/terms";
import type { Optional } from "./types/effects";

/** Columns of an import file, in order, matching `dev/sample-vocab.csv`.
 *  The `notes` column must be present (it may be empty); anything after it
 *  — including further delimiters — is part of the notes. */
const MIN_FIELDS = 4;

/** A successfully parsed line: the 1-based line number (so the UI can
 *  point the learner at it) and the Term it yielded. */
export type ParsedRow = { line: number; term: NewTerm };

/** A line that could not be parsed, with the reason and its 1-based
 *  number as it appears in the file. */
export type ParseError = { line: number; reason: string };

export type ParseResult = { rows: ParsedRow[]; errors: ParseError[] };

/** Split `text` into Terms on `delimiter`. Blank lines are ignored;
 *  every other line must have at least {@link MIN_FIELDS} fields with the
 *  three identity fields non-empty, or it lands in `errors` with its line
 *  number. The first three fields are the identity; everything after them
 *  is re-joined on `delimiter` as free-text `notes`, so a delimiter
 *  inside a note is fine. Order is preserved and never throws. */
export const parseVocab = (text: string, delimiter: string): ParseResult => {
  const rows: ParsedRow[] = [];
  const errors: ParseError[] = [];

  text.split(/\r?\n/).forEach((raw, index) => {
    const line = index + 1;
    if (raw.trim().length === 0) return;

    const fields = raw.split(delimiter);
    if (fields.length < MIN_FIELDS) {
      errors.push({
        line,
        reason: `expected at least ${MIN_FIELDS} columns, found ${fields.length}`,
      });
      return;
    }

    const [foreignLang, foreignText, pivotText] = fields.map((f) => f.trim());
    const notes = fields.slice(3).join(delimiter).trim();
    if (
      foreignLang.length === 0 ||
      foreignText.length === 0 ||
      pivotText.length === 0
    ) {
      errors.push({
        line,
        reason: "foreign_lang, foreign_text and pivot_text must not be empty",
      });
      return;
    }

    rows.push({
      line,
      term: {
        foreign_lang: foreignLang,
        foreign_text: foreignText,
        pivot_text: pivotText,
        notes: blankToUndefined(notes),
      },
    });
  });

  return { rows, errors };
};

/** The Import button is live only once there is something to send and
 *  nothing the parser choked on. */
export const canImport = (result: ParseResult): boolean =>
  result.rows.length > 0 && result.errors.length === 0;

/** The Terms a parsed file contributes to `POST /terms/import`, in file
 *  order. Keeps `ParsedRow` shape knowledge inside this module. */
export const termsToImport = (result: ParseResult): NewTerm[] =>
  result.rows.map((row) => row.term);

const blankToUndefined = (value: string): Optional<string> =>
  value.length === 0 ? undefined : value;
