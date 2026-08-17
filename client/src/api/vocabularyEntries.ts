import type { BulkImportRow } from '../bulkImport'
import type { Result } from '../types/effects'
import { apiPost, type ApiError } from './client'

export type VocabularyEntry = {
  id: number
  source_language: string
  source_text: string
  target_language: string
  target_text: string
  category_ids: number[]
  created_at: string
  updated_at: string
}

export type NewVocabularyEntry = {
  sourceLanguage: string
  sourceText: string
  targetLanguage: string
  targetText: string
  categoryIds: number[]
}

export function createVocabularyEntry(
  entry: NewVocabularyEntry,
): Promise<Result<VocabularyEntry, ApiError>> {
  return apiPost<VocabularyEntry>('/api/vocabulary-entries', {
    source_language: entry.sourceLanguage,
    source_text: entry.sourceText,
    target_language: entry.targetLanguage,
    target_text: entry.targetText,
    category_ids: entry.categoryIds,
  })
}

/**
 * Atomically creates every parsed bulk row, sharing one source/target
 * Language Pair across the whole paste; each row's Category is resolved by
 * normalized name on the server (spec.md story 24, 25, 26; ticket 06).
 */
export function bulkCreateVocabularyEntries(
  sourceLanguage: string,
  targetLanguage: string,
  rows: BulkImportRow[],
): Promise<Result<VocabularyEntry[], ApiError>> {
  return apiPost<VocabularyEntry[]>('/api/vocabulary-entries/bulk', {
    entries: rows.map((row) => ({
      source_language: sourceLanguage,
      source_text: row.sourceText,
      target_language: targetLanguage,
      target_text: row.targetText,
      category_name: row.categoryName,
    })),
  })
}
