import { describe, expect, it } from 'vitest'
import { parsePipeDelimitedRows } from './bulkImport'

describe('parsePipeDelimitedRows', () => {
  // The client parses the pasted `source | target | category` convenience
  // format so the server accepts JSON only (spec.md story 24; ticket 06).
  it('parses multiple rows, ignoring blank lines', () => {
    const result = parsePipeDelimitedRows('manzana | apple | Fruit\n\nperro | dog | Animals')

    expect(result).toEqual({
      ok: true,
      value: [
        { sourceText: 'manzana', targetText: 'apple', categoryName: 'Fruit' },
        { sourceText: 'perro', targetText: 'dog', categoryName: 'Animals' },
      ],
    })
  })

  it('trims surrounding whitespace within a row', () => {
    const result = parsePipeDelimitedRows('  manzana  |  apple  |  Fruit  ')

    expect(result).toEqual({
      ok: true,
      value: [{ sourceText: 'manzana', targetText: 'apple', categoryName: 'Fruit' }],
    })
  })

  it('rejects a row with the wrong number of fields', () => {
    const result = parsePipeDelimitedRows('manzana | apple')

    expect(result.ok).toBe(false)
    if (!result.ok) {
      expect(result.error).toEqual([{ line: 1, reason: 'Expected "source | target | category"' }])
    }
  })

  it('rejects a row with an empty field', () => {
    const result = parsePipeDelimitedRows('manzana |  | Fruit')

    expect(result.ok).toBe(false)
  })

  it('rejects empty input', () => {
    const result = parsePipeDelimitedRows('   ')

    expect(result.ok).toBe(false)
  })
})
