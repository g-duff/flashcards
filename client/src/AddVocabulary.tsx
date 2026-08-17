import { useEffect, useState, type SubmitEvent } from 'react'
import { type ApiError } from './api/client'
import { listCategories, type Category } from './api/categories'
import { bulkCreateVocabularyEntries, createVocabularyEntry } from './api/vocabularyEntries'
import { parseBulkCsv } from './bulkImport'

type CategoriesScreen =
  | { kind: 'loading' }
  | { kind: 'loaded'; categories: Category[] }
  | { kind: 'error'; message: string }

type Feedback = { kind: 'idle' } | { kind: 'submitting' } | { kind: 'error'; message: string }

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

const AddVocabulary = () => {
  const [categoriesScreen, setCategoriesScreen] = useState<CategoriesScreen>({ kind: 'loading' })
  const [sourceLanguage, setSourceLanguage] = useState('')
  const [sourceText, setSourceText] = useState('')
  const [targetLanguage, setTargetLanguage] = useState('')
  const [targetText, setTargetText] = useState('')
  const [selectedCategoryIds, setSelectedCategoryIds] = useState<number[]>([])
  const [feedback, setFeedback] = useState<Feedback>({ kind: 'idle' })

  const [bulkSourceLanguage, setBulkSourceLanguage] = useState('')
  const [bulkTargetLanguage, setBulkTargetLanguage] = useState('')
  const [bulkCsvText, setBulkCsvText] = useState('')
  const [bulkFeedback, setBulkFeedback] = useState<Feedback>({ kind: 'idle' })

  const bulkParseResult = parseBulkCsv(bulkCsvText)

  useEffect(() => {
    let cancelled = false

    loadCategories().then((nextScreen) => {
      if (!cancelled) setCategoriesScreen(nextScreen)
    })

    return () => {
      cancelled = true
    }
  }, [])

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
      setBulkCsvText('')
      setBulkFeedback({ kind: 'idle' })
      return
    }

    setBulkFeedback({ kind: 'error', message: bulkErrorMessage(result.error) })
  }

  return (
    <section className="add-vocabulary">
      <h2>Add Vocabulary</h2>

      {categoriesScreen.kind === 'loading' && <p>Loading categories…</p>}
      {categoriesScreen.kind === 'error' && <p role="alert">{categoriesScreen.message}</p>}

      {categoriesScreen.kind === 'loaded' && (
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

      {feedback.kind === 'error' && <p role="alert">{feedback.message}</p>}

      <h2>Bulk import</h2>
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

        <label htmlFor="bulk-csv">Paste rows as "source | target | category"</label>
        <textarea
          id="bulk-csv"
          value={bulkCsvText}
          onChange={(event) => setBulkCsvText(event.target.value)}
        />

        {bulkCsvText.trim().length > 0 && !bulkParseResult.ok && (
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
            bulkCsvText.trim().length === 0 ||
            bulkSourceLanguage.trim().length === 0 ||
            bulkTargetLanguage.trim().length === 0
          }
        >
          Import rows
        </button>
      </form>

      {bulkFeedback.kind === 'error' && <p role="alert">{bulkFeedback.message}</p>}
    </section>
  )
}

export default AddVocabulary
