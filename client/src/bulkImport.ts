import type { Result } from './types/effects'

/** One parsed row of the `source <delimiter> target <delimiter> category` convenience format. */
export type BulkImportRow = {
  sourceText: string
  targetText: string
  categoryName: string
}

export type BulkImportRowError = {
  line: number
  reason: string
}

/** A delimiter offered in the bulk-import UI, with a human-readable label. */
export type BulkImportDelimiterOption = {
  label: string
  value: string
}

export const BULK_IMPORT_DELIMITER_OPTIONS: BulkImportDelimiterOption[] = [
  { label: 'Bar ( | )', value: '|' },
  { label: 'Comma ( , )', value: ',' },
  { label: 'Semicolon ( ; )', value: ';' },
  { label: 'Tab', value: '\t' },
]

export const DEFAULT_BULK_IMPORT_DELIMITER = '|'

/**
 * Parses pasted `source <delimiter> target <delimiter> category` rows
 * (spec.md story 24, 30; grilled-spec.md sec. 3). The delimiter is
 * caller-chosen — this is delimiter-separated text, not CSV, so there is no
 * comma splitting or quote handling. Blank lines are ignored. Any malformed
 * row fails the whole parse — the caller cannot submit a partially valid
 * paste.
 */
export function parseDelimitedRows(
  raw: string,
  delimiter: string,
): Result<BulkImportRow[], BulkImportRowError[]> {
  const lines = raw
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0)

  if (lines.length === 0) {
    return { ok: false, error: [{ line: 0, reason: 'Paste at least one row' }] }
  }

  const expectedShape = `Expected "source ${delimiter} target ${delimiter} category"`

  const { rows, errors } = lines.reduce<{
    rows: BulkImportRow[]
    errors: BulkImportRowError[]
  }>(
    (accumulator, line, index) => {
      const fields = line.split(delimiter).map((field) => field.trim())
      if (fields.length !== 3 || fields.some((field) => field.length === 0)) {
        const error = { line: index + 1, reason: expectedShape }
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
