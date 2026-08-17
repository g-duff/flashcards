import { useEffect, useState, type SubmitEvent } from 'react'
import {
  createCategory,
  deleteCategory,
  listCategories,
  renameCategory,
  type Category,
  type CategorySort,
} from './api/categories'

type Screen =
  | { kind: 'loading' }
  | { kind: 'loaded'; categories: Category[] }
  | { kind: 'error'; message: string }

type Feedback = { kind: 'idle' } | { kind: 'submitting' } | { kind: 'error'; message: string }

async function loadCategories(sort: CategorySort): Promise<Screen> {
  const result = await listCategories(sort)
  return result.ok
    ? { kind: 'loaded', categories: result.value }
    : { kind: 'error', message: result.error.message }
}

type CategoriesProps = { onChanged?: () => void }

const Categories = ({ onChanged }: CategoriesProps) => {
  const [sort, setSort] = useState<CategorySort>('created_at')
  const [screen, setScreen] = useState<Screen>({ kind: 'loading' })
  const [newName, setNewName] = useState('')
  const [feedback, setFeedback] = useState<Feedback>({ kind: 'idle' })
  const [renameValues, setRenameValues] = useState<Record<number, string>>({})

  useEffect(() => {
    let cancelled = false

    loadCategories(sort).then((nextScreen) => {
      if (!cancelled) setScreen(nextScreen)
    })

    return () => {
      cancelled = true
    }
  }, [sort])

  async function refresh() {
    setScreen(await loadCategories(sort))
    onChanged?.()
  }

  async function handleCreate(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault()
    setFeedback({ kind: 'submitting' })

    const result = await createCategory(newName)
    if (result.ok) {
      setNewName('')
      setFeedback({ kind: 'idle' })
      await refresh()
      return
    }

    setFeedback({ kind: 'error', message: result.error.message })
  }

  async function handleRename(category: Category) {
    const nextName = renameValues[category.id] ?? category.name
    setFeedback({ kind: 'submitting' })

    const result = await renameCategory(category.id, nextName)
    if (result.ok) {
      setFeedback({ kind: 'idle' })
      await refresh()
      return
    }

    setFeedback({ kind: 'error', message: result.error.message })
  }

  async function handleDelete(category: Category) {
    setFeedback({ kind: 'submitting' })

    const result = await deleteCategory(category.id)
    if (result.ok) {
      setFeedback({ kind: 'idle' })
      await refresh()
      return
    }

    setFeedback({ kind: 'error', message: result.error.message })
  }

  return (
    <section className="categories">
      <h2>Categories</h2>

      <label htmlFor="category-sort">Sort by</label>
      <select
        id="category-sort"
        value={sort}
        onChange={(event) => setSort(event.target.value as CategorySort)}
      >
        <option value="created_at">Creation date</option>
        <option value="alphabetical">Alphabetical</option>
      </select>

      {screen.kind === 'loading' && <p>Loading categories…</p>}
      {screen.kind === 'error' && <p role="alert">{screen.message}</p>}

      {screen.kind === 'loaded' && (
        <ul>
          {screen.categories.map((category) => (
            <li key={category.id}>
              <input
                aria-label={`Rename ${category.name}`}
                value={renameValues[category.id] ?? category.name}
                onChange={(event) =>
                  setRenameValues({ ...renameValues, [category.id]: event.target.value })
                }
              />
              <button
                type="button"
                onClick={() => handleRename(category)}
                disabled={feedback.kind === 'submitting'}
              >
                Save name
              </button>
              <button
                type="button"
                onClick={() => handleDelete(category)}
                disabled={feedback.kind === 'submitting'}
              >
                Delete
              </button>
            </li>
          ))}
        </ul>
      )}

      <form onSubmit={handleCreate}>
        <label htmlFor="category-name">New category name</label>
        <input
          id="category-name"
          value={newName}
          onChange={(event) => setNewName(event.target.value)}
        />
        <button type="submit" disabled={feedback.kind === 'submitting'}>
          Create category
        </button>
      </form>

      {feedback.kind === 'error' && <p role="alert">{feedback.message}</p>}
    </section>
  )
}

export default Categories
