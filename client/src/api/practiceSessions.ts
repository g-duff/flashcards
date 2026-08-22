import type { Optional, Result } from '../types/effects'
import { apiGet, apiPost, type ApiError } from './client'

export type TranslationDirection = 'source_to_target' | 'target_to_source'

export type SessionStatus = 'active' | 'completed'

export type PublicOption = {
  id: number
  text: string
  is_dont_know: boolean
}

export type PracticeQuestion = {
  id: number
  vocabulary_entry_id: number
  direction: TranslationDirection
  ordinal: number
  prompt_text: string
  options: PublicOption[]
}

export type PracticeSession = {
  id: number
  learner_id: number
  category_id: number
  direction: TranslationDirection
  status: SessionStatus
  requested_question_count: number
  actual_question_count: number
  answered_question_count: number
  correct_count: number
  started_at: string
  completed_at: Optional<string>
  last_activity_at: string
  created_at: string
  updated_at: string
  questions: PracticeQuestion[]
}

/** The server's JSON shape: `completed_at` is `null`, not omitted, until a
 * session completes (grilled-spec.md sec. 9). */
type RawPracticeSession = Omit<PracticeSession, 'completed_at'> & { completed_at: string | null }

function normalizeSession(raw: RawPracticeSession): PracticeSession {
  return { ...raw, completed_at: raw.completed_at ?? undefined }
}

export async function createPracticeSession(
  categoryId: number,
  direction: TranslationDirection,
  questionCount: number,
): Promise<Result<PracticeSession, ApiError>> {
  const result = await apiPost<RawPracticeSession>('/api/practice-sessions', {
    category_id: categoryId,
    direction,
    question_count: questionCount,
  })
  return result.ok ? { ok: true, value: normalizeSession(result.value) } : result
}

export async function getPracticeSession(
  sessionId: number,
): Promise<Result<PracticeSession, ApiError>> {
  const result = await apiGet<RawPracticeSession>(`/api/practice-sessions/${sessionId}`)
  return result.ok ? { ok: true, value: normalizeSession(result.value) } : result
}
