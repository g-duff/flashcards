import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import Categories from './Categories'
import { createCategory, deleteCategory, listCategories } from './api/categories'
import { apiError } from './testUtils/envelopes'

vi.mock('./api/categories')

const animals = {
  id: 1,
  name: 'Animals',
  created_at: '2026-08-12T00:00:00Z',
  updated_at: '2026-08-12T00:00:00Z',
}

describe('Categories', () => {
  beforeEach(() => {
    vi.resetAllMocks()
  })

  // Categories screen: list Categories sorted per the default order
  // (spec.md story 9; ticket 04).
  it('lists categories returned by the server', async () => {
    vi.mocked(listCategories).mockResolvedValue({ ok: true, value: [animals] })

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
    vi.mocked(listCategories).mockResolvedValue({ ok: true, value: [animals] })
    vi.mocked(deleteCategory).mockResolvedValue({
      ok: false,
      error: apiError('That category could not be deleted.'),
    })

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
    vi.mocked(listCategories).mockResolvedValueOnce({ ok: true, value: [] })
    vi.mocked(createCategory).mockResolvedValue({ ok: true, value: created })

    render(<Categories />)

    await waitFor(() => expect(listCategories).toHaveBeenCalledTimes(1))

    // The refresh triggered by a successful create refetches the list.
    vi.mocked(listCategories).mockResolvedValue({ ok: true, value: [created] })

    fireEvent.change(screen.getByLabelText('New category name'), {
      target: { value: 'Fruits' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Create category' }))

    expect(await screen.findByDisplayValue('Fruits')).toBeInTheDocument()
    expect(createCategory).toHaveBeenCalledWith('Fruits')
  })
})
