import { afterEach, describe, expect, it, vi } from 'vitest'
import { errorEnvelope, jsonResponse, successEnvelope } from '../testUtils/envelopes'
import { apiDelete, apiGet, apiPatch, apiPost, ApiError } from './client'

function stubFetch(handler: (url: string, init?: RequestInit) => unknown) {
  const fetchMock = vi
    .fn()
    .mockImplementation((url: string, init?: RequestInit) => jsonResponse(handler(url, init)))
  vi.stubGlobal('fetch', fetchMock)
  return fetchMock
}

describe('api/client', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('resolves to an ok Result with the envelope data on success', async () => {
    stubFetch(() => successEnvelope({ status: 'ok' }))

    const result = await apiGet('/api/health')

    expect(result).toEqual({ ok: true, value: { status: 'ok' } })
  })

  it('resolves to an error Result carrying the server message and envelope on failure', async () => {
    stubFetch(() => errorEnvelope('CONFLICT', 'That category could not be deleted.'))

    const result = await apiGet('/api/categories/1')

    expect(result.ok).toBe(false)
    if (result.ok) throw new Error('expected an error Result')
    expect(result.error).toBeInstanceOf(ApiError)
    expect(result.error.message).toBe('That category could not be deleted.')
    expect(result.error.envelope?.error.code).toBe('CONFLICT')
  })

  it('treats a null data field as an undefined value', async () => {
    stubFetch(() => successEnvelope(null))

    const result = await apiDelete('/api/categories/1')

    expect(result).toEqual({ ok: true, value: undefined })
  })

  it('resolves to an error Result when the network request itself fails', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('network error')))

    const result = await apiGet('/api/health')

    expect(result.ok).toBe(false)
    if (result.ok) throw new Error('expected an error Result')
    expect(result.error.message).toBe('network error')
    expect(result.error.envelope).toBeUndefined()
  })

  it('sends a JSON body with the given method and credentials for a POST', async () => {
    const fetchMock = stubFetch(() => successEnvelope({ id: 1 }))

    await apiPost('/api/categories', { name: 'Fruit' })

    expect(fetchMock).toHaveBeenCalledWith('/api/categories', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Fruit' }),
      credentials: 'include',
    })
  })

  it('sends the PATCH method with a JSON body', async () => {
    const fetchMock = stubFetch(() => successEnvelope({ id: 1 }))

    await apiPatch('/api/categories/1', { name: 'Fruit' })

    expect(fetchMock).toHaveBeenCalledWith('/api/categories/1', {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Fruit' }),
      credentials: 'include',
    })
  })

  it('sends the DELETE method with no body', async () => {
    const fetchMock = stubFetch(() => successEnvelope(null))

    await apiDelete('/api/categories/1')

    expect(fetchMock).toHaveBeenCalledWith('/api/categories/1', {
      method: 'DELETE',
      credentials: 'include',
    })
  })
})
