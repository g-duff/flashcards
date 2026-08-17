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
 * grilled-spec.md sec. 3). Blank lines are ignored. Any malformed row fails
 * the whole parse — the caller cannot submit a partially valid paste.
 */
export function parseBulkCsv(raw: string): Result<BulkImportRow[], BulkImportRowError[]> {
  const lines = raw
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0)

  if (lines.length === 0) {
    return { ok: false, error: [{ line: 0, reason: 'Paste at least one row' }] }
  }

  const errors: BulkImportRowError[] = []
  const rows: BulkImportRow[] = []

  lines.forEach((line, index) => {
    const fields = line.split('|').map((field) => field.trim())
    if (fields.length !== 3 || fields.some((field) => field.length === 0)) {
      errors.push({ line: index + 1, reason: 'Expected "source | target | category"' })
      return
    }

    const [sourceText, targetText, categoryName] = fields as [string, string, string]
    rows.push({ sourceText, targetText, categoryName })
  })

  return errors.length > 0 ? { ok: false, error: errors } : { ok: true, value: rows }
}
