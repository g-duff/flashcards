import { describe, expect, it } from 'vitest'
import { parseDelimitedRows } from './bulkImport'

describe('parseDelimitedRows', () => {
  // The client parses the pasted `source <delimiter> target <delimiter>
  // category` convenience format so the server accepts JSON only (spec.md
  // story 24; ticket 06).
  it('parses multiple rows, ignoring blank lines', () => {
    const result = parseDelimitedRows('manzana | apple | Fruit\n\nperro | dog | Animals', '|')

    expect(result).toEqual({
      ok: true,
      value: [
        { sourceText: 'manzana', targetText: 'apple', categoryName: 'Fruit' },
        { sourceText: 'perro', targetText: 'dog', categoryName: 'Animals' },
      ],
    })
  })

  it('trims surrounding whitespace within a row', () => {
    const result = parseDelimitedRows('  manzana  |  apple  |  Fruit  ', '|')

    expect(result).toEqual({
      ok: true,
      value: [{ sourceText: 'manzana', targetText: 'apple', categoryName: 'Fruit' }],
    })
  })

  // The delimiter is caller-chosen, defaulting to the bar in the UI, but
  // any single- or multi-character delimiter works (ticket 06 follow-up).
  it('parses rows using a caller-chosen delimiter', () => {
    const result = parseDelimitedRows('manzana, apple, Fruit', ',')

    expect(result).toEqual({
      ok: true,
      value: [{ sourceText: 'manzana', targetText: 'apple', categoryName: 'Fruit' }],
    })
  })

  it('rejects a row with the wrong number of fields', () => {
    const result = parseDelimitedRows('manzana | apple', '|')

    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error).toEqual([{ line: 1, reason: 'Expected "source | target | category"' }])
    }
  })

  it('rejects a row with an empty field', () => {
    const result = parseDelimitedRows('manzana |  | Fruit', '|')

    expect(result.ok).toBe(false)
  })

  it('rejects empty input', () => {
    const result = parseDelimitedRows('   ', '|')

    expect(result.ok).toBe(false)
  })
})
