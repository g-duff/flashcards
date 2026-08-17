import { describe, expect, it } from 'vitest'
import { parseBulkCsv } from './bulkImport'

describe('parseBulkCsv', () => {
  // The client parses the pasted `source | target | category` convenience
  // format so the server accepts JSON only (spec.md story 24; ticket 06).
  it('parses multiple rows, ignoring blank lines', () => {
    const result = parseBulkCsv('manzana | apple | Fruit\n\nperro | dog | Animals')

    expect(result).toEqual({
      ok: true,
      value: [
        { sourceText: 'manzana', targetText: 'apple', categoryName: 'Fruit' },
        { sourceText: 'perro', targetText: 'dog', categoryName: 'Animals' },
      ],
    })
  })

  it('trims surrounding whitespace within a row', () => {
    const result = parseBulkCsv('  manzana  |  apple  |  Fruit  ')

    expect(result).toEqual({
      ok: true,
      value: [{ sourceText: 'manzana', targetText: 'apple', categoryName: 'Fruit' }],
    })
  })

  it('rejects a row with the wrong number of fields', () => {
    const result = parseBulkCsv('manzana | apple')

    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error).toEqual([{ line: 1, reason: 'Expected "source | target | category"' }])
    }
  })

  it('rejects a row with an empty field', () => {
    const result = parseBulkCsv('manzana |  | Fruit')

    expect(result.ok).toBe(false)
  })

  it('rejects empty input', () => {
    const result = parseBulkCsv('   ')

    expect(result.ok).toBe(false)
  })
})
