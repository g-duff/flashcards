import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import Categories from './Categories'

function jsonResponse(body: unknown) {
  return Promise.resolve({ json: () => Promise.resolve(body) } as Response)
}

function successEnvelope(data: unknown) {
  return { status: 'success', data, meta: { timestamp: '2026-08-12T00:00:00Z' } }
}

function errorEnvelope(code: string, message: string) {
  return {
    status: 'error',
    error: { code, message, details: [] },
    meta: { timestamp: '2026-08-12T00:00:00Z' },
  }
}

const animals = {
  id: 1,
  name: 'Animals',
  created_at: '2026-08-12T00:00:00Z',
  updated_at: '2026-08-12T00:00:00Z',
}

describe('Categories', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  // Categories screen: list Categories sorted per the default order
  // (spec.md story 9; ticket 04).
  it('lists categories returned by the server', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockImplementation(() => jsonResponse(successEnvelope([animals]))),
    )

    render(<Categories />)

    expect(await screen.findByDisplayValue('Animals')).toBeInTheDocument()
  })

  // The client surfaces whatever error message the server returns for a
  // rejected deletion (ticket 04). The server does not yet reject deletions
  // (that lands with the orphan-check in ticket 05, once Vocabulary Entries
  // exist), so this exercises the generic error-rendering path with a
  // representative conflict response rather than a code the server emits
  // today.
  it('shows the server error message when a deletion request fails', async () => {
    const fetchMock = vi
      .fn()
      .mockImplementationOnce(() => jsonResponse(successEnvelope([animals])))
      .mockImplementationOnce(() =>
        jsonResponse(errorEnvelope('CONFLICT', 'That category could not be deleted.')),
      )
    vi.stubGlobal('fetch', fetchMock)

    render(<Categories />)

    const deleteButton = await screen.findByRole('button', { name: 'Delete' })
    fireEvent.click(deleteButton)

    expect(await screen.findByText('That category could not be deleted.')).toBeInTheDocument()
  })

  // Create flow: submitting the create form calls the API and refreshes
  // the list (spec.md story 8; ticket 04).
  it('creates a category and refreshes the list', async () => {
    const created = {
      id: 2,
      name: 'Fruits',
      created_at: '2026-08-12T00:00:00Z',
      updated_at: '2026-08-12T00:00:00Z',
    }
    const fetchMock = vi
      .fn()
      .mockImplementationOnce(() => jsonResponse(successEnvelope([])))
      .mockImplementationOnce(() => jsonResponse(successEnvelope(created)))
      .mockImplementationOnce(() => jsonResponse(successEnvelope([created])))
    vi.stubGlobal('fetch', fetchMock)

    render(<Categories />)

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1))

    fireEvent.change(screen.getByLabelText('New category name'), {
      target: { value: 'Fruits' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Create category' }))

    expect(await screen.findByDisplayValue('Fruits')).toBeInTheDocument()
  })
})
