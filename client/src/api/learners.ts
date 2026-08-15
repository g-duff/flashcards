import type { Optional, Result } from '../types/effects'
import { apiDelete, apiGet, apiPatch, apiPost, type ApiError } from './client'

export type Learner = {
  id: number
  name: string
  created_at: string
  updated_at: string
}

export function createLearner(name: string): Promise<Result<Learner, ApiError>> {
  return apiPost<Learner>('/api/learners', { name })
}

export function listLearners(): Promise<Result<Learner[], ApiError>> {
  return apiGet<Learner[]>('/api/learners')
}

export function selectLearner(learnerId: number): Promise<Result<Learner, ApiError>> {
  return apiPost<Learner>('/api/session/learner', { learner_id: learnerId })
}

export function getCurrentLearner(): Promise<Result<Optional<Learner>, ApiError>> {
  return apiGet<Optional<Learner>>('/api/session/learner')
}

export function clearLearnerSession(): Promise<Result<undefined, ApiError>> {
  return apiDelete<undefined>('/api/session/learner')
}

export function renameLearner(learnerId: number, name: string): Promise<Result<Learner, ApiError>> {
  return apiPatch<Learner>(`/api/learners/${learnerId}`, { name })
}

export function deleteLearner(learnerId: number): Promise<Result<undefined, ApiError>> {
  return apiDelete<undefined>(`/api/learners/${learnerId}`)
}
