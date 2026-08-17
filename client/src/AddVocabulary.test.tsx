import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import AddVocabulary from './AddVocabulary'
import { listCategories } from './api/categories'
import { bulkCreateVocabularyEntries, createVocabularyEntry } from './api/vocabularyEntries'
import { apiError } from './testUtils/envelopes'

vi.mock('./api/categories')
vi.mock('./api/vocabularyEntries')

const fruit = {
  id: 1,
  name: 'Fruit',
  created_at: '2026-08-12T00:00:00Z',
  updated_at: '2026-08-12T00:00:00Z',
}

function switchToBulkImport() {
  fireEvent.click(screen.getByRole('tab', { name: 'Bulk import' }))
}

const manzana = {
  id: 10,
  source_language: 'es',
  source_text: 'manzana',
  target_language: 'en',
  target_text: 'apple',
  category_ids: [1],
  created_at: '2026-08-12T00:00:00Z',
  updated_at: '2026-08-12T00:00:00Z',
}

describe('AddVocabulary', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    vi.mocked(listCategories).mockResolvedValue({ ok: true, value: [fruit] })
  })

  // Add Vocabulary screen: create a single Vocabulary Entry with source and
  // target text, languages, and at least one Category (spec.md story 13,
  // 16, 18; ticket 05).
  it('creates a vocabulary entry from the form', async () => {
    vi.mocked(createVocabularyEntry).mockResolvedValue({ ok: true, value: manzana })

    render(<AddVocabulary />)

    await screen.findByText('Fruit')

    fireEvent.change(screen.getByLabelText('Source language'), { target: { value: 'es' } })
    fireEvent.change(screen.getByLabelText('Source text'), { target: { value: 'manzana' } })
    fireEvent.change(screen.getByLabelText('Target language'), { target: { value: 'en' } })
    fireEvent.change(screen.getByLabelText('Target text'), { target: { value: 'apple' } })
    fireEvent.click(screen.getByLabelText('Fruit'))
    fireEvent.click(screen.getByRole('button', { name: 'Add vocabulary entry' }))

    await waitFor(() => expect(createVocabularyEntry).toHaveBeenCalledTimes(1))
    expect(createVocabularyEntry).toHaveBeenCalledWith({
      sourceLanguage: 'es',
      sourceText: 'manzana',
      targetLanguage: 'en',
      targetText: 'apple',
      categoryIds: [1],
    })
  })

  // Duplicate and validation errors surface from the server (spec.md story
  // 15, 20; ticket 05).
  it('shows the server error message when creation is rejected', async () => {
    vi.mocked(createVocabularyEntry).mockResolvedValue({
      ok: false,
      error: apiError('A vocabulary entry already exists.'),
    })

    render(<AddVocabulary />)

    await screen.findByText('Fruit')

    fireEvent.change(screen.getByLabelText('Source language'), { target: { value: 'es' } })
    fireEvent.change(screen.getByLabelText('Source text'), { target: { value: 'manzana' } })
    fireEvent.change(screen.getByLabelText('Target language'), { target: { value: 'en' } })
    fireEvent.change(screen.getByLabelText('Target text'), { target: { value: 'apple' } })
    fireEvent.click(screen.getByLabelText('Fruit'))
    fireEvent.click(screen.getByRole('button', { name: 'Add vocabulary entry' }))

    expect(await screen.findByText('A vocabulary entry already exists.')).toBeInTheDocument()
  })

  // Bulk import parses pasted `source | target | category` rows and commits
  // them atomically via the bulk endpoint (spec.md story 24, 25; ticket 06).
  it('bulk imports parsed rows atomically', async () => {
    const perro = {
      id: 11,
      source_language: 'es',
      source_text: 'perro',
      target_language: 'en',
      target_text: 'dog',
      category_ids: [1],
      created_at: '2026-08-12T00:00:00Z',
      updated_at: '2026-08-12T00:00:00Z',
    }
    vi.mocked(bulkCreateVocabularyEntries).mockResolvedValue({ ok: true, value: [manzana, perro] })

    render(<AddVocabulary />)

    await screen.findByText('Fruit')
    switchToBulkImport()

    fireEvent.change(screen.getByLabelText('Bulk source language'), {
      target: { value: 'es' },
    })
    fireEvent.change(screen.getByLabelText('Bulk target language'), {
      target: { value: 'en' },
    })
    fireEvent.change(screen.getByLabelText('Paste rows as "source | target | category"'), {
      target: { value: 'manzana | apple | Fruit\nperro | dog | Fruit' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Import rows' }))

    await waitFor(() => expect(bulkCreateVocabularyEntries).toHaveBeenCalledTimes(1))
    expect(bulkCreateVocabularyEntries).toHaveBeenCalledWith('es', 'en', [
      { sourceText: 'manzana', targetText: 'apple', categoryName: 'Fruit' },
      { sourceText: 'perro', targetText: 'dog', categoryName: 'Fruit' },
    ])
  })

  // The delimiter is selectable in the UI, defaulting to the bar (ticket 06
  // follow-up).
  it('parses and submits bulk rows using a selected non-default delimiter', async () => {
    vi.mocked(bulkCreateVocabularyEntries).mockResolvedValue({ ok: true, value: [manzana] })

    render(<AddVocabulary />)

    await screen.findByText('Fruit')
    switchToBulkImport()

    fireEvent.change(screen.getByLabelText('Delimiter'), { target: { value: ',' } })
    fireEvent.change(screen.getByLabelText('Bulk source language'), { target: { value: 'es' } })
    fireEvent.change(screen.getByLabelText('Bulk target language'), { target: { value: 'en' } })
    fireEvent.change(screen.getByLabelText('Paste rows as "source , target , category"'), {
      target: { value: 'manzana, apple, Fruit' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Import rows' }))

    await waitFor(() => expect(bulkCreateVocabularyEntries).toHaveBeenCalledTimes(1))
    expect(bulkCreateVocabularyEntries).toHaveBeenCalledWith('es', 'en', [
      { sourceText: 'manzana', targetText: 'apple', categoryName: 'Fruit' },
    ])
  })

  // An unparseable row is shown before the client attempts to submit
  // (spec.md story 26; ticket 06).
  it('shows a parse error and disables submit for a malformed bulk row', async () => {
    render(<AddVocabulary />)

    await screen.findByText('Fruit')
    switchToBulkImport()

    fireEvent.change(screen.getByLabelText('Paste rows as "source | target | category"'), {
      target: { value: 'manzana | apple' },
    })

    expect(await screen.findByText(/Expected "source \| target \| category"/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Import rows' })).toBeDisabled()
    expect(bulkCreateVocabularyEntries).not.toHaveBeenCalled()
  })

  // A rejected bulk import surfaces the server's error message and leaves
  // no partial rows implied by success (spec.md story 26; ticket 06).
  it('shows the server error message when bulk import is rejected', async () => {
    vi.mocked(bulkCreateVocabularyEntries).mockResolvedValue({
      ok: false,
      error: apiError('Bulk import references a category that does not exist.'),
    })

    render(<AddVocabulary />)

    await screen.findByText('Fruit')
    switchToBulkImport()

    fireEvent.change(screen.getByLabelText('Bulk source language'), { target: { value: 'es' } })
    fireEvent.change(screen.getByLabelText('Bulk target language'), { target: { value: 'en' } })
    fireEvent.change(screen.getByLabelText('Paste rows as "source | target | category"'), {
      target: { value: 'manzana | apple | Unknown' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Import rows' }))

    expect(
      await screen.findByText('Bulk import references a category that does not exist.'),
    ).toBeInTheDocument()
  })

  // Field-level detail from the server envelope is folded into the shown
  // message so a Learner knows which pasted row failed (spec.md story 73;
  // ticket 06).
  it('includes the failing row field in the bulk import error message', async () => {
    vi.mocked(bulkCreateVocabularyEntries).mockResolvedValue({
      ok: false,
      error: apiError('Bulk import references a category that does not exist.', [
        { field: 'entries[1].category_name', reason: 'No category with this name exists' },
      ]),
    })

    render(<AddVocabulary />)

    await screen.findByText('Fruit')
    switchToBulkImport()

    fireEvent.change(screen.getByLabelText('Bulk source language'), { target: { value: 'es' } })
    fireEvent.change(screen.getByLabelText('Bulk target language'), { target: { value: 'en' } })
    fireEvent.change(screen.getByLabelText('Paste rows as "source | target | category"'), {
      target: { value: 'manzana | apple | Fruit\nperro | dog | Unknown' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Import rows' }))

    expect(
      await screen.findByText(
        'Bulk import references a category that does not exist. (entries[1].category_name: No category with this name exists)',
      ),
    ).toBeInTheDocument()
  })

  // Switching between single-entry and bulk-import tabs must not discard
  // in-progress input in the tab left behind.
  it('preserves typed single-entry values when switching tabs and back', async () => {
    render(<AddVocabulary />)

    await screen.findByText('Fruit')

    fireEvent.change(screen.getByLabelText('Source text'), { target: { value: 'manzana' } })

    switchToBulkImport()
    fireEvent.click(screen.getByRole('tab', { name: 'Single entry' }))

    expect(screen.getByLabelText('Source text')).toHaveValue('manzana')
  })
})
