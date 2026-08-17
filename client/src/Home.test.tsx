import { fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import Home from './Home'

function jsonResponse(body: unknown) {
  return Promise.resolve({ json: () => Promise.resolve(body) } as Response)
}

function successEnvelope(data: unknown) {
  return { status: 'success', data, meta: { timestamp: '2026-08-12T00:00:00Z' } }
}

const alice = {
  id: 1,
  name: 'Alice',
  created_at: '2026-08-12T00:00:00Z',
  updated_at: '2026-08-12T00:00:00Z',
}
const bea = {
  id: 2,
  name: 'Bea',
  created_at: '2026-08-12T00:00:00Z',
  updated_at: '2026-08-12T00:00:00Z',
}
const animals = {
  id: 1,
  name: 'Animals',
  created_at: '2026-08-12T00:00:00Z',
  updated_at: '2026-08-12T00:00:00Z',
}

/**
 * Routes a mocked fetch call by URL substring, falling back to an empty
 * list. Keeps these tests resilient to unrelated fetches fired by
 * components mounted alongside the one under test (e.g. Categories, which
 * fetches on mount whenever a learner is selected).
 */
function routedFetch(routes: Record<string, unknown>) {
  return vi.fn().mockImplementation((url: string) => {
    const match = Object.entries(routes).find(([path]) => url.includes(path))
    return jsonResponse(successEnvelope(match ? match[1] : []))
  })
}

describe('Home', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  // Home screen: display the current Learner when the cookie already
  // resolves one (spec.md story 3; ticket 02).
  it('shows the current learner when the session cookie already resolves one', async () => {
    vi.stubGlobal('fetch', routedFetch({ '/api/session/learner': alice }))

    render(<Home />)

    expect(await screen.findByText('Alice')).toBeInTheDocument()
  })

  // An invalid or deleted-profile cookie is cleared server-side, so
  // GET /api/session/learner resolves to null data; the client must show
  // the Home profile-choosing screen rather than getting stuck (spec.md
  // story 4; ticket 02).
  it('shows the Home profile-choosing screen when the session resolves to no learner', async () => {
    vi.stubGlobal('fetch', routedFetch({ '/api/session/learner': null }))

    render(<Home />)

    expect(await screen.findByRole('heading', { name: 'Choose a profile' })).toBeInTheDocument()
    expect(await screen.findByLabelText('New learner name')).toBeInTheDocument()
  })

  // Home screen: create a Learner and it becomes the current Learner
  // (spec.md story 1; ticket 02).
  it('lets a learner create a new profile and become the current learner', async () => {
    const fetchMock = routedFetch({ '/api/session/learner': null, '/api/learners': bea })
    vi.stubGlobal('fetch', fetchMock)

    render(<Home />)

    const input = await screen.findByLabelText('New learner name')
    fireEvent.change(input, { target: { value: 'Bea' } })
    fireEvent.click(screen.getByRole('button', { name: 'Create profile' }))

    expect(await screen.findByText('Bea')).toBeInTheDocument()
    const createCall = fetchMock.mock.calls.find(
      (call) => call[1]?.method === 'POST' && call[0].includes('/api/learners'),
    )
    expect(createCall).toBeDefined()
  })

  // Home screen: select an existing Learner and it becomes the current
  // Learner (spec.md story 2; ticket 02).
  it('lets a learner select an existing profile', async () => {
    const fetchMock = routedFetch({
      '/api/session/learner': null,
      '/api/learners': [alice],
    })
    fetchMock.mockImplementation((url: string, init?: RequestInit) => {
      if (url.includes('/api/session/learner') && init?.method === 'POST') {
        return jsonResponse(successEnvelope(alice))
      }
      if (url.includes('/api/session/learner')) return jsonResponse(successEnvelope(null))
      if (url.includes('/api/learners')) return jsonResponse(successEnvelope([alice]))
      return jsonResponse(successEnvelope([]))
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<Home />)

    const selectButton = await screen.findByRole('button', { name: 'Alice' })
    fireEvent.click(selectButton)

    expect(await screen.findByRole('button', { name: 'Switch profile' })).toBeInTheDocument()
    const selectCall = fetchMock.mock.calls.find(
      (call) => call[1]?.method === 'POST' && call[0].includes('/api/session/learner'),
    )
    expect(selectCall).toBeDefined()
  })

  // Duplicate-name conflicts surface as inline feedback rather than
  // silently failing (spec.md story 6; ticket 02).
  it('shows an error when creating a learner with a duplicate name', async () => {
    const fetchMock = vi.fn().mockImplementation((url: string, init?: RequestInit) => {
      if (url.includes('/api/session/learner')) return jsonResponse(successEnvelope(null))
      if (url.includes('/api/learners') && init?.method === 'POST') {
        return jsonResponse({
          status: 'error',
          error: {
            code: 'LEARNER_NAME_CONFLICT',
            message: 'A learner with this name already exists.',
            details: [{ field: 'name', reason: 'Already in use' }],
          },
          meta: { timestamp: '2026-08-12T00:00:00Z' },
        })
      }
      return jsonResponse(successEnvelope([]))
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<Home />)

    const input = await screen.findByLabelText('New learner name')
    fireEvent.change(input, { target: { value: 'Alice' } })
    fireEvent.click(screen.getByRole('button', { name: 'Create profile' }))

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'A learner with this name already exists.',
    )
  })

  // A Learner can rename their profile without losing identity (spec.md
  // story 5; ticket 03).
  it('lets the current learner rename their profile', async () => {
    const renamed = { ...alice, name: 'Alicia' }
    const fetchMock = vi.fn().mockImplementation((url: string, init?: RequestInit) => {
      if (url.includes('/api/learners/1') && init?.method === 'PATCH') {
        return jsonResponse(successEnvelope(renamed))
      }
      if (url.includes('/api/session/learner')) return jsonResponse(successEnvelope(alice))
      return jsonResponse(successEnvelope([]))
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<Home />)

    const input = await screen.findByLabelText('Rename profile')
    fireEvent.change(input, { target: { value: 'Alicia' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save name' }))

    expect(await screen.findByText('Alicia')).toBeInTheDocument()
    const renameCall = fetchMock.mock.calls.find(
      (call) => call[1]?.method === 'PATCH' && call[0].includes('/api/learners/1'),
    )
    expect(renameCall).toBeDefined()
  })

  // A Learner can deliberately delete their profile, so their personal data
  // is removed (spec.md story 7; ticket 03).
  it('lets the current learner delete their profile after confirming', async () => {
    let deleted = false
    const fetchMock = vi.fn().mockImplementation((url: string, init?: RequestInit) => {
      if (url.includes('/api/learners/1') && init?.method === 'DELETE') {
        deleted = true
        return jsonResponse(successEnvelope(null))
      }
      if (url.includes('/api/session/learner')) {
        return jsonResponse(successEnvelope(deleted ? null : alice))
      }
      return jsonResponse(successEnvelope([]))
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<Home />)

    fireEvent.click(await screen.findByRole('button', { name: 'Delete profile' }))
    fireEvent.click(screen.getByRole('button', { name: 'Confirm delete' }))

    expect(await screen.findByRole('heading', { name: 'Choose a profile' })).toBeInTheDocument()
    const deleteCall = fetchMock.mock.calls.find(
      (call) => call[1]?.method === 'DELETE' && call[0].includes('/api/learners/1'),
    )
    expect(deleteCall).toBeDefined()
  })

  // Cancelling the delete confirmation leaves the profile untouched.
  it('lets the current learner cancel a delete confirmation', async () => {
    const fetchMock = vi.fn().mockImplementation((url: string) => {
      if (url.includes('/api/session/learner')) return jsonResponse(successEnvelope(alice))
      return jsonResponse(successEnvelope([]))
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<Home />)

    fireEvent.click(await screen.findByRole('button', { name: 'Delete profile' }))
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))

    expect(screen.queryByRole('button', { name: 'Confirm delete' })).not.toBeInTheDocument()
    const writeCall = fetchMock.mock.calls.find((call) => call[1]?.method !== undefined)
    expect(writeCall).toBeUndefined()
  })

  // Categories and Add Vocabulary each fetch their own category list, so
  // creating a category must signal Add Vocabulary to refetch rather than
  // leaving its checkboxes stale until a page reload.
  it('shows a newly created category in Add Vocabulary without reloading', async () => {
    let categoryCreated = false
    const fetchMock = vi.fn().mockImplementation((url: string, init?: RequestInit) => {
      if (url.includes('/api/session/learner')) return jsonResponse(successEnvelope(alice))
      if (url.includes('/api/categories') && init?.method === 'POST') {
        categoryCreated = true
        return jsonResponse(successEnvelope(animals))
      }
      if (url.includes('/api/categories')) {
        return jsonResponse(successEnvelope(categoryCreated ? [animals] : []))
      }
      return jsonResponse(successEnvelope([]))
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<Home />)

    await screen.findByText('Alice')
    expect(screen.queryByLabelText('Animals')).not.toBeInTheDocument()

    fireEvent.change(screen.getByLabelText('New category name'), { target: { value: 'Animals' } })
    fireEvent.click(screen.getByRole('button', { name: 'Create category' }))

    expect(await screen.findByLabelText('Animals')).toBeInTheDocument()
  })
})
