import type { Result } from '../types/effects'
import { apiDelete, apiGet, apiPatch, apiPost, type ApiError } from './client'

export type Category = {
  id: number
  name: string
  created_at: string
  updated_at: string
}

export type CategorySort = 'created_at' | 'alphabetical'

export function createCategory(name: string): Promise<Result<Category, ApiError>> {
  return apiPost<Category>('/api/categories', { name })
}

export function listCategories(
  sort: CategorySort = 'created_at',
): Promise<Result<Category[], ApiError>> {
  return apiGet<Category[]>(`/api/categories?sort=${sort}`)
}

export function renameCategory(
  categoryId: number,
  name: string,
): Promise<Result<Category, ApiError>> {
  return apiPatch<Category>(`/api/categories/${categoryId}`, { name })
}

export function deleteCategory(categoryId: number): Promise<Result<undefined, ApiError>> {
  return apiDelete<undefined>(`/api/categories/${categoryId}`)
}
