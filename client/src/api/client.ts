import type { Optional, Result } from '../types/effects'

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? ''

export interface SuccessEnvelope<T> {
  status: 'success'
  data: T | null
  meta: { timestamp: string }
}

export interface ErrorEnvelope {
  status: 'error'
  error: {
    code: string
    message: string
    details: { field: string; reason: string }[]
  }
  meta: { timestamp: string }
}

export class ApiError extends Error {
  envelope: Optional<ErrorEnvelope>

  constructor(message: string, envelope?: ErrorEnvelope) {
    super(message)
    this.envelope = envelope
  }
}

async function apiRequest<T>(path: string, init: RequestInit): Promise<Result<T, ApiError>> {
  let response: Response
  let body: SuccessEnvelope<T> | ErrorEnvelope

  try {
    response = await fetch(`${API_BASE_URL}${path}`, { ...init, credentials: 'include' })
    body = (await response.json()) as SuccessEnvelope<T> | ErrorEnvelope
  } catch (cause) {
    const message = cause instanceof Error ? cause.message : 'Network request failed'
    return { ok: false, error: new ApiError(message) }
  }

  if (body.status === 'error') {
    return { ok: false, error: new ApiError(body.error.message, body) }
  }

  return { ok: true, value: (body.data ?? undefined) as T }
}

export function apiGet<T>(path: string): Promise<Result<T, ApiError>> {
  return apiRequest<T>(path, {})
}

export function apiPost<T>(path: string, body: unknown): Promise<Result<T, ApiError>> {
  return apiRequest<T>(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
}

export function apiDelete<T>(path: string): Promise<Result<T, ApiError>> {
  return apiRequest<T>(path, { method: 'DELETE' })
}

export function apiPatch<T>(path: string, body: unknown): Promise<Result<T, ApiError>> {
  return apiRequest<T>(path, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
}
