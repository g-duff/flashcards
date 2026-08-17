import type { Result } from './types/effects'

/** One parsed row of the `source | target | category` convenience format. */
export type BulkImportRow = {
  sourceText: string
  targetText: string
  categoryName: string
}

export type BulkImportRowError = {
  line: number
  reason: string
}

/**
 * Parses pasted `source | target | category` rows (spec.md story 24, 30;
 * grilled-spec.md sec. 3). Pipe-delimited, not CSV — no comma splitting or
 * quote handling. Blank lines are ignored. Any malformed row fails the whole
 * parse — the caller cannot submit a partially valid paste.
 */
export function parsePipeDelimitedRows(raw: string): Result<BulkImportRow[], BulkImportRowError[]> {
  const lines = raw
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0)

  if (lines.length === 0) {
    return { ok: false, error: [{ line: 0, reason: 'Paste at least one row' }] }
  }

  const { rows, errors } = lines.reduce<{
    rows: BulkImportRow[]
    errors: BulkImportRowError[]
  }>(
    (accumulator, line, index) => {
      const fields = line.split('|').map((field) => field.trim())
      if (fields.length !== 3 || fields.some((field) => field.length === 0)) {
        const error = { line: index + 1, reason: 'Expected "source | target | category"' }
        return { ...accumulator, errors: [...accumulator.errors, error] }
      }

      const [sourceText, targetText, categoryName] = fields as [string, string, string]
      const row = { sourceText, targetText, categoryName }
      return { ...accumulator, rows: [...accumulator.rows, row] }
    },
    { rows: [], errors: [] },
  )

  return errors.length > 0 ? { ok: false, error: errors } : { ok: true, value: rows }
}
