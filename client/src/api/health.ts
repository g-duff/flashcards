import type { Result } from '../types/effects'
import { apiGet, type ApiError } from './client'

export interface HealthData {
  status: string
}

export function getHealth(): Promise<Result<HealthData, ApiError>> {
  return apiGet<HealthData>('/api/health')
}
