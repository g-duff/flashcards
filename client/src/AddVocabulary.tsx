import { useEffect, useState, type SubmitEvent } from 'react'
import { listCategories, type Category } from './api/categories'
import { createVocabularyEntry } from './api/vocabularyEntries'

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

const AddVocabulary = () => {
  const [categoriesScreen, setCategoriesScreen] = useState<CategoriesScreen>({ kind: 'loading' })
  const [sourceLanguage, setSourceLanguage] = useState('')
  const [sourceText, setSourceText] = useState('')
  const [targetLanguage, setTargetLanguage] = useState('')
  const [targetText, setTargetText] = useState('')
  const [selectedCategoryIds, setSelectedCategoryIds] = useState<number[]>([])
  const [feedback, setFeedback] = useState<Feedback>({ kind: 'idle' })

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
    </section>
  )
}

export default AddVocabulary
