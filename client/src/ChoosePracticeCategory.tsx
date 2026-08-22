import { useEffect, useState, type SubmitEvent } from 'react'
import { listCategories, type Category } from './api/categories'
import {
  createPracticeSession,
  type PracticeSession,
  type TranslationDirection,
} from './api/practiceSessions'

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

type ChoosePracticeCategoryProps = {
  onSessionStarted: (session: PracticeSession) => void
}

const DEFAULT_QUESTION_COUNT = 10

const ChoosePracticeCategory = ({ onSessionStarted }: ChoosePracticeCategoryProps) => {
  const [categoriesScreen, setCategoriesScreen] = useState<CategoriesScreen>({ kind: 'loading' })
  const [categoryId, setCategoryId] = useState<number | undefined>(undefined)
  const [direction, setDirection] = useState<TranslationDirection>('source_to_target')
  const [questionCount, setQuestionCount] = useState(DEFAULT_QUESTION_COUNT)
  const [feedback, setFeedback] = useState<Feedback>({ kind: 'idle' })

  useEffect(() => {
    let cancelled = false

    loadCategories().then((nextScreen) => {
      if (cancelled) return
      setCategoriesScreen(nextScreen)
      if (nextScreen.kind === 'loaded' && nextScreen.categories.length > 0) {
        setCategoryId((current) => current ?? nextScreen.categories[0]?.id)
      }
    })

    return () => {
      cancelled = true
    }
  }, [])

  async function handleStart(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault()
    if (categoryId === undefined) return

    setFeedback({ kind: 'submitting' })

    const result = await createPracticeSession(categoryId, direction, questionCount)
    if (result.ok) {
      setFeedback({ kind: 'idle' })
      onSessionStarted(result.value)
      return
    }

    setFeedback({ kind: 'error', message: result.error.message })
  }

  return (
    <section className="choose-practice-category">
      <h2>Choose Practice Category</h2>

      {categoriesScreen.kind === 'loading' && <p>Loading categories…</p>}
      {categoriesScreen.kind === 'error' && <p role="alert">{categoriesScreen.message}</p>}

      {categoriesScreen.kind === 'loaded' && categoriesScreen.categories.length === 0 && (
        <p>Add a category and some vocabulary before starting a practice session.</p>
      )}

      {categoriesScreen.kind === 'loaded' && categoriesScreen.categories.length > 0 && (
        <form onSubmit={handleStart}>
          <label htmlFor="practice-category">Category</label>
          <select
            id="practice-category"
            value={categoryId}
            onChange={(event) => setCategoryId(Number(event.target.value))}
          >
            {categoriesScreen.categories.map((category) => (
              <option key={category.id} value={category.id}>
                {category.name}
              </option>
            ))}
          </select>

          <fieldset>
            <legend>Direction</legend>
            <label>
              <input
                type="radio"
                name="direction"
                value="source_to_target"
                checked={direction === 'source_to_target'}
                onChange={() => setDirection('source_to_target')}
              />
              Source → Target
            </label>
            <label>
              <input
                type="radio"
                name="direction"
                value="target_to_source"
                checked={direction === 'target_to_source'}
                onChange={() => setDirection('target_to_source')}
              />
              Target → Source
            </label>
          </fieldset>

          <label htmlFor="practice-question-count">Question count</label>
          <input
            id="practice-question-count"
            type="number"
            value={questionCount}
            onChange={(event) => setQuestionCount(Number(event.target.value))}
          />

          <button type="submit" disabled={feedback.kind === 'submitting'}>
            Start practice
          </button>
        </form>
      )}

      {feedback.kind === 'error' && <p role="alert">{feedback.message}</p>}
    </section>
  )
}

export default ChoosePracticeCategory
