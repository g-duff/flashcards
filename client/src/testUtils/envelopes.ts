import { ApiError, type ErrorEnvelope } from '../api/client'

export function jsonResponse(body: unknown): Promise<Response> {
  return Promise.resolve({ json: () => Promise.resolve(body) } as Response)
}

export function successEnvelope(data: unknown) {
  return { status: 'success', data, meta: { timestamp: '2026-08-12T00:00:00Z' } }
}

export function errorEnvelope(code: string, message: string, details: unknown[] = []) {
  return {
    status: 'error',
    error: { code, message, details },
    meta: { timestamp: '2026-08-12T00:00:00Z' },
  }
}

/** An ApiError carrying an envelope, for stubbing a rejected api/* call. */
export function apiError(
  message: string,
  details: ErrorEnvelope['error']['details'] = [],
): ApiError {
  return new ApiError(message, errorEnvelope('ERROR', message, details) as ErrorEnvelope)
}
