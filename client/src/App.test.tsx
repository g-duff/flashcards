import { render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App'
import { getHealth } from './api/health'
import { getCurrentLearner, listLearners } from './api/learners'
import { apiError } from './testUtils/envelopes'

vi.mock('./api/health')
vi.mock('./api/learners')

describe('App', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    // App always renders Home, which resolves the current learner on mount;
    // default to no learner so these health-check tests don't also need to
    // stub the Categories/Add Vocabulary subtree.
    vi.mocked(getCurrentLearner).mockResolvedValue({ ok: true, value: undefined })
    vi.mocked(listLearners).mockResolvedValue({ ok: true, value: [] })
  })

  // As a developer, calling the server health endpoint proves the client
  // scaffold can talk to the server (01-project-scaffolding.md).
  it('shows the server status once the health check succeeds', async () => {
    vi.mocked(getHealth).mockResolvedValue({ ok: true, value: { status: 'ok' } })

    render(<App />)

    expect(await screen.findByText('Server is ok')).toBeInTheDocument()
  })

  it('reports the server as unreachable when the health check fails', async () => {
    vi.mocked(getHealth).mockResolvedValue({ ok: false, error: apiError('network error') })

    render(<App />)

    expect(await screen.findByText('Server is unreachable')).toBeInTheDocument()
  })
})
