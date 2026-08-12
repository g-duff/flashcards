const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080'

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
  envelope: ErrorEnvelope

  constructor(envelope: ErrorEnvelope) {
    super(envelope.error.message)
    this.envelope = envelope
  }
}

export async function apiGet<T>(path: string): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    credentials: 'include',
  })
  const body = (await response.json()) as SuccessEnvelope<T> | ErrorEnvelope

  if (body.status === 'error') {
    throw new ApiError(body)
  }

  return body.data as T
}
