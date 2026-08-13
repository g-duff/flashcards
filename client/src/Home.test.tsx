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
    vi.stubGlobal(
      'fetch',
      vi.fn().mockImplementation(() => jsonResponse(successEnvelope(alice))),
    )

    render(<Home />)

    expect(await screen.findByText('Alice')).toBeInTheDocument()
  })

  // An invalid or deleted-profile cookie is cleared server-side, so
  // GET /api/session/learner resolves to null data; the client must show
  // the Home profile-choosing screen rather than getting stuck (spec.md
  // story 4; ticket 02).
  it('shows the Home profile-choosing screen when the session resolves to no learner', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockImplementation(() => jsonResponse(successEnvelope(null))),
    )

    render(<Home />)

    expect(await screen.findByRole('heading', { name: 'Choose a profile' })).toBeInTheDocument()
    expect(await screen.findByLabelText('New learner name')).toBeInTheDocument()
  })

  // Home screen: create a Learner and it becomes the current Learner
  // (spec.md story 1; ticket 02).
  it('lets a learner create a new profile and become the current learner', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve(successEnvelope(null)) })
      .mockResolvedValueOnce({ json: () => Promise.resolve(successEnvelope([])) })
      .mockResolvedValueOnce({ json: () => Promise.resolve(successEnvelope(bea)) })
    vi.stubGlobal('fetch', fetchMock)

    render(<Home />)

    const input = await screen.findByLabelText('New learner name')
    fireEvent.change(input, { target: { value: 'Bea' } })
    fireEvent.click(screen.getByRole('button', { name: 'Create profile' }))

    expect(await screen.findByText('Bea')).toBeInTheDocument()
    expect(fetchMock.mock.calls[2][0]).toContain('/api/learners')
    expect(fetchMock.mock.calls[2][1]?.method).toBe('POST')
  })

  // Home screen: select an existing Learner and it becomes the current
  // Learner (spec.md story 2; ticket 02).
  it('lets a learner select an existing profile', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve(successEnvelope(null)) })
      .mockResolvedValueOnce({ json: () => Promise.resolve(successEnvelope([alice])) })
      .mockResolvedValueOnce({ json: () => Promise.resolve(successEnvelope(alice)) })
    vi.stubGlobal('fetch', fetchMock)

    render(<Home />)

    const selectButton = await screen.findByRole('button', { name: 'Alice' })
    fireEvent.click(selectButton)

    expect(await screen.findByRole('button', { name: 'Switch profile' })).toBeInTheDocument()
    expect(fetchMock.mock.calls[2][0]).toContain('/api/session/learner')
  })

  // Duplicate-name conflicts surface as inline feedback rather than
  // silently failing (spec.md story 6; ticket 02).
  it('shows an error when creating a learner with a duplicate name', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve(successEnvelope(null)) })
      .mockResolvedValueOnce({ json: () => Promise.resolve(successEnvelope([])) })
      .mockResolvedValueOnce({
        json: () =>
          Promise.resolve({
            status: 'error',
            error: {
              code: 'LEARNER_NAME_CONFLICT',
              message: 'A learner with this name already exists.',
              details: [{ field: 'name', reason: 'Already in use' }],
            },
            meta: { timestamp: '2026-08-12T00:00:00Z' },
          }),
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
})
