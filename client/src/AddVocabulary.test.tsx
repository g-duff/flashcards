import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import AddVocabulary from './AddVocabulary'

function jsonResponse(body: unknown) {
  return Promise.resolve({ json: () => Promise.resolve(body) } as Response)
}

function successEnvelope(data: unknown) {
  return { status: 'success', data, meta: { timestamp: '2026-08-12T00:00:00Z' } }
}

function errorEnvelope(code: string, message: string, details: unknown[] = []) {
  return {
    status: 'error',
    error: { code, message, details },
    meta: { timestamp: '2026-08-12T00:00:00Z' },
  }
}

const fruit = {
  id: 1,
  name: 'Fruit',
  created_at: '2026-08-12T00:00:00Z',
  updated_at: '2026-08-12T00:00:00Z',
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
    vi.restoreAllMocks()
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  // Add Vocabulary screen: create a single Vocabulary Entry with source and
  // target text, languages, and at least one Category (spec.md story 13,
  // 16, 18; ticket 05).
  it('creates a vocabulary entry from the form', async () => {
    const fetchMock = vi
      .fn()
      .mockImplementationOnce(() => jsonResponse(successEnvelope([fruit])))
      .mockImplementationOnce(() => jsonResponse(successEnvelope(manzana)))
    vi.stubGlobal('fetch', fetchMock)

    render(<AddVocabulary />)

    await screen.findByText('Fruit')

    fireEvent.change(screen.getByLabelText('Source language'), { target: { value: 'es' } })
    fireEvent.change(screen.getByLabelText('Source text'), { target: { value: 'manzana' } })
    fireEvent.change(screen.getByLabelText('Target language'), { target: { value: 'en' } })
    fireEvent.change(screen.getByLabelText('Target text'), { target: { value: 'apple' } })
    fireEvent.click(screen.getByLabelText('Fruit'))
    fireEvent.click(screen.getByRole('button', { name: 'Add vocabulary entry' }))

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2))
    const body = JSON.parse(String(fetchMock.mock.calls[1]?.[1]?.body))
    expect(body).toMatchObject({
      source_language: 'es',
      source_text: 'manzana',
      target_language: 'en',
      target_text: 'apple',
      category_ids: [1],
    })
  })

  // Duplicate and validation errors surface from the server (spec.md story
  // 15, 20; ticket 05).
  it('shows the server error message when creation is rejected', async () => {
    const fetchMock = vi
      .fn()
      .mockImplementationOnce(() => jsonResponse(successEnvelope([fruit])))
      .mockImplementationOnce(() =>
        jsonResponse(
          errorEnvelope('VOCABULARY_ENTRY_CONFLICT', 'A vocabulary entry already exists.'),
        ),
      )
    vi.stubGlobal('fetch', fetchMock)

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
    const fetchMock = vi
      .fn()
      .mockImplementationOnce(() => jsonResponse(successEnvelope([fruit])))
      .mockImplementationOnce(() => jsonResponse(successEnvelope([manzana, perro])))
    vi.stubGlobal('fetch', fetchMock)

    render(<AddVocabulary />)

    await screen.findByText('Fruit')

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

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2))
    const body = JSON.parse(String(fetchMock.mock.calls[1]?.[1]?.body))
    expect(body).toEqual({
      entries: [
        {
          source_language: 'es',
          source_text: 'manzana',
          target_language: 'en',
          target_text: 'apple',
          category_name: 'Fruit',
        },
        {
          source_language: 'es',
          source_text: 'perro',
          target_language: 'en',
          target_text: 'dog',
          category_name: 'Fruit',
        },
      ],
    })
  })

  // An unparseable row is shown before the client attempts to submit
  // (spec.md story 26; ticket 06).
  it('shows a parse error and disables submit for a malformed bulk row', async () => {
    const fetchMock = vi.fn().mockImplementationOnce(() => jsonResponse(successEnvelope([fruit])))
    vi.stubGlobal('fetch', fetchMock)

    render(<AddVocabulary />)

    await screen.findByText('Fruit')

    fireEvent.change(screen.getByLabelText('Paste rows as "source | target | category"'), {
      target: { value: 'manzana | apple' },
    })

    expect(await screen.findByText(/Expected "source \| target \| category"/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Import rows' })).toBeDisabled()
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  // A rejected bulk import surfaces the server's error message and leaves
  // no partial rows implied by success (spec.md story 26; ticket 06).
  it('shows the server error message when bulk import is rejected', async () => {
    const fetchMock = vi
      .fn()
      .mockImplementationOnce(() => jsonResponse(successEnvelope([fruit])))
      .mockImplementationOnce(() =>
        jsonResponse(
          errorEnvelope(
            'UNKNOWN_CATEGORY',
            'Bulk import references a category that does not exist.',
          ),
        ),
      )
    vi.stubGlobal('fetch', fetchMock)

    render(<AddVocabulary />)

    await screen.findByText('Fruit')

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
    const fetchMock = vi
      .fn()
      .mockImplementationOnce(() => jsonResponse(successEnvelope([fruit])))
      .mockImplementationOnce(() =>
        jsonResponse(
          errorEnvelope(
            'UNKNOWN_CATEGORY',
            'Bulk import references a category that does not exist.',
            [{ field: 'entries[1].category_name', reason: 'No category with this name exists' }],
          ),
        ),
      )
    vi.stubGlobal('fetch', fetchMock)

    render(<AddVocabulary />)

    await screen.findByText('Fruit')

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
})
