import { apiGet } from './client'

export interface HealthData {
  status: string
}

export function getHealth(): Promise<HealthData> {
  return apiGet<HealthData>('/api/health')
}
