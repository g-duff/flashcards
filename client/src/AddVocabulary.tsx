import { useEffect, useState, type SubmitEvent } from 'react'
import { type ApiError } from './api/client'
import { listCategories, type Category } from './api/categories'
import { bulkCreateVocabularyEntries, createVocabularyEntry } from './api/vocabularyEntries'
import {
  BULK_IMPORT_DELIMITER_OPTIONS,
  DEFAULT_BULK_IMPORT_DELIMITER,
  parseDelimitedRows,
} from './bulkImport'

type CategoriesScreen =
  | { kind: 'loading' }
  | { kind: 'loaded'; categories: Category[] }
  | { kind: 'error'; message: string }

type Feedback = { kind: 'idle' } | { kind: 'submitting' } | { kind: 'error'; message: string }

type EntryMode = 'single' | 'bulk'

async function loadCategories(): Promise<CategoriesScreen> {
  const result = await listCategories()
  return result.ok
    ? { kind: 'loaded', categories: result.value }
    : { kind: 'error', message: result.error.message }
}

/**
 * Bulk import spans several rows, so the generic error message alone
 * cannot tell a Learner which pasted row failed. Fold in the server's
 * per-row field detail (spec.md story 73; ticket 06) when present.
 */
function bulkErrorMessage(error: ApiError): string {
  const details = error.envelope?.error.details ?? []
  if (details.length === 0) return error.message
  return `${error.message} (${details.map((detail) => `${detail.field}: ${detail.reason}`).join('; ')})`
}

type AddVocabularyProps = { categoriesVersion?: number }

const AddVocabulary = ({ categoriesVersion }: AddVocabularyProps) => {
  const [categoriesScreen, setCategoriesScreen] = useState<CategoriesScreen>({ kind: 'loading' })
  const [entryMode, setEntryMode] = useState<EntryMode>('single')
  const [sourceLanguage, setSourceLanguage] = useState('')
  const [sourceText, setSourceText] = useState('')
  const [targetLanguage, setTargetLanguage] = useState('')
  const [targetText, setTargetText] = useState('')
  const [selectedCategoryIds, setSelectedCategoryIds] = useState<number[]>([])
  const [feedback, setFeedback] = useState<Feedback>({ kind: 'idle' })

  const [bulkSourceLanguage, setBulkSourceLanguage] = useState('')
  const [bulkTargetLanguage, setBulkTargetLanguage] = useState('')
  const [bulkPasteText, setBulkPasteText] = useState('')
  const [bulkDelimiter, setBulkDelimiter] = useState(DEFAULT_BULK_IMPORT_DELIMITER)
  const [bulkFeedback, setBulkFeedback] = useState<Feedback>({ kind: 'idle' })

  const bulkParseResult = parseDelimitedRows(bulkPasteText, bulkDelimiter)

  useEffect(() => {
    let cancelled = false

    loadCategories().then((nextScreen) => {
      if (!cancelled) setCategoriesScreen(nextScreen)
    })

    return () => {
      cancelled = true
    }
  }, [categoriesVersion])

  function toggleCategory(categoryId: number) {
    setSelectedCategoryIds((current) =>
      current.includes(categoryId)
        ? current.filter((id) => id !== categoryId)
        : [...current, categoryId],
    )
  }

  async function handleCreate(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault()
    setFeedback({ kind: 'submitting' })

    const result = await createVocabularyEntry({
      sourceLanguage,
      sourceText,
      targetLanguage,
      targetText,
      categoryIds: selectedCategoryIds,
    })

    if (result.ok) {
      setSourceText('')
      setTargetText('')
      setSelectedCategoryIds([])
      setFeedback({ kind: 'idle' })
      return
    }

    setFeedback({ kind: 'error', message: result.error.message })
  }

  async function handleBulkImport(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!bulkParseResult.ok) return

    setBulkFeedback({ kind: 'submitting' })

    const result = await bulkCreateVocabularyEntries(
      bulkSourceLanguage,
      bulkTargetLanguage,
      bulkParseResult.value,
    )

    if (result.ok) {
      setBulkPasteText('')
      setBulkFeedback({ kind: 'idle' })
      return
    }

    setBulkFeedback({ kind: 'error', message: bulkErrorMessage(result.error) })
  }

  return (
    <section className="add-vocabulary">
      <h2>Add Vocabulary</h2>

      <div className="entry-mode-toggle" role="tablist" aria-label="Vocabulary entry mode">
        <button
          type="button"
          role="tab"
          aria-selected={entryMode === 'single'}
          className={entryMode === 'single' ? 'active' : undefined}
          onClick={() => setEntryMode('single')}
        >
          Single entry
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={entryMode === 'bulk'}
          className={entryMode === 'bulk' ? 'active' : undefined}
          onClick={() => setEntryMode('bulk')}
        >
          Bulk import
        </button>
      </div>

      {categoriesScreen.kind === 'loading' && <p>Loading categories…</p>}
      {categoriesScreen.kind === 'error' && <p role="alert">{categoriesScreen.message}</p>}

      {entryMode === 'single' && categoriesScreen.kind === 'loaded' && (
        <form onSubmit={handleCreate}>
          <label htmlFor="source-language">Source language</label>
          <input
            id="source-language"
            value={sourceLanguage}
            onChange={(event) => setSourceLanguage(event.target.value)}
          />

          <label htmlFor="source-text">Source text</label>
          <input
            id="source-text"
            value={sourceText}
            onChange={(event) => setSourceText(event.target.value)}
          />

          <label htmlFor="target-language">Target language</label>
          <input
            id="target-language"
            value={targetLanguage}
            onChange={(event) => setTargetLanguage(event.target.value)}
          />

          <label htmlFor="target-text">Target text</label>
          <input
            id="target-text"
            value={targetText}
            onChange={(event) => setTargetText(event.target.value)}
          />

          <fieldset>
            <legend>Categories</legend>
            {categoriesScreen.categories.length === 0 && <p>Create a category first.</p>}
            {categoriesScreen.categories.map((category) => (
              <label key={category.id}>
                <input
                  type="checkbox"
                  checked={selectedCategoryIds.includes(category.id)}
                  onChange={() => toggleCategory(category.id)}
                />
                {category.name}
              </label>
            ))}
          </fieldset>

          <button type="submit" disabled={feedback.kind === 'submitting'}>
            Add vocabulary entry
          </button>
        </form>
      )}

      {entryMode === 'single' && feedback.kind === 'error' && (
        <p role="alert">{feedback.message}</p>
      )}

      {entryMode === 'bulk' && (
        <form onSubmit={handleBulkImport}>
          <label htmlFor="bulk-source-language">Bulk source language</label>
          <input
            id="bulk-source-language"
            value={bulkSourceLanguage}
            onChange={(event) => setBulkSourceLanguage(event.target.value)}
          />

          <label htmlFor="bulk-target-language">Bulk target language</label>
          <input
            id="bulk-target-language"
            value={bulkTargetLanguage}
            onChange={(event) => setBulkTargetLanguage(event.target.value)}
          />

          <label htmlFor="bulk-delimiter">Delimiter</label>
          <select
            id="bulk-delimiter"
            value={bulkDelimiter}
            onChange={(event) => setBulkDelimiter(event.target.value)}
          >
            {BULK_IMPORT_DELIMITER_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>

          <label htmlFor="bulk-rows">
            Paste rows as "source {bulkDelimiter} target {bulkDelimiter} category"
          </label>
          <textarea
            id="bulk-rows"
            value={bulkPasteText}
            onChange={(event) => setBulkPasteText(event.target.value)}
          />

          {bulkPasteText.trim().length > 0 && !bulkParseResult.ok && (
            <ul role="alert">
              {bulkParseResult.error.map((rowError) => (
                <li key={rowError.line}>
                  Line {rowError.line}: {rowError.reason}
                </li>
              ))}
            </ul>
          )}

          <button
            type="submit"
            disabled={
              bulkFeedback.kind === 'submitting' ||
              !bulkParseResult.ok ||
              bulkPasteText.trim().length === 0 ||
              bulkSourceLanguage.trim().length === 0 ||
              bulkTargetLanguage.trim().length === 0
            }
          >
            Import rows
          </button>
        </form>
      )}

      {entryMode === 'bulk' && bulkFeedback.kind === 'error' && (
        <p role="alert">{bulkFeedback.message}</p>
      )}
    </section>
  )
}

export default AddVocabulary
