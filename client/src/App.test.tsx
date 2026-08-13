import { render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App'

function mockFetchOnce(body: unknown) {
  vi.stubGlobal(
    'fetch',
    vi.fn().mockResolvedValue({
      json: () => Promise.resolve(body),
    }),
  )
}

describe('App', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  // As a developer, calling the server health endpoint proves the client
  // scaffold can talk to the server (01-project-scaffolding.md).
  it('shows the server status once the health check succeeds', async () => {
    mockFetchOnce({
      status: 'success',
      data: { status: 'ok' },
      meta: { timestamp: '2026-08-12T00:00:00Z' },
    })

    render(<App />)

    expect(await screen.findByText('Server is ok')).toBeInTheDocument()
  })

  it('reports the server as unreachable when the health check fails', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('network error')))

    render(<App />)

    expect(await screen.findByText('Server is unreachable')).toBeInTheDocument()
  })
})
