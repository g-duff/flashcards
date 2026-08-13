import { useEffect, useState, type SubmitEvent } from 'react'
import {
  clearLearnerSession,
  createLearner,
  getCurrentLearner,
  listLearners,
  selectLearner,
  type Learner,
} from './api/learners'

type Screen =
  | { kind: 'loading' }
  | { kind: 'selected'; learner: Learner }
  | { kind: 'choosing'; learners: Learner[] }

type Feedback = { kind: 'idle' } | { kind: 'submitting' } | { kind: 'error'; message: string }

async function loadChoosingScreen(): Promise<Screen> {
  const result = await listLearners()
  return { kind: 'choosing', learners: (result.ok ? result.value : undefined) ?? [] }
}

const Home = () => {
  const [screen, setScreen] = useState<Screen>({ kind: 'loading' })
  const [newName, setNewName] = useState('')
  const [feedback, setFeedback] = useState<Feedback>({ kind: 'idle' })

  useEffect(() => {
    let cancelled = false

    async function loadInitialScreen() {
      const currentResult = await getCurrentLearner()
      if (cancelled) return

      const current = currentResult.ok ? currentResult.value : undefined
      if (current) {
        setScreen({ kind: 'selected', learner: current })
        return
      }

      const nextScreen = await loadChoosingScreen()
      if (cancelled) return
      setScreen(nextScreen)
    }

    loadInitialScreen()

    return () => {
      cancelled = true
    }
  }, [])

  async function handleCreate(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault()
    setFeedback({ kind: 'submitting' })

    const result = await createLearner(newName)
    if (result.ok) {
      setNewName('')
      setFeedback({ kind: 'idle' })
      setScreen({ kind: 'selected', learner: result.value })
      return
    }

    setFeedback({ kind: 'error', message: result.error.message })
  }

  async function handleSelect(learnerId: number) {
    setFeedback({ kind: 'submitting' })

    const result = await selectLearner(learnerId)
    if (result.ok) {
      setFeedback({ kind: 'idle' })
      setScreen({ kind: 'selected', learner: result.value })
      return
    }

    setFeedback({ kind: 'error', message: result.error.message })
  }

  async function handleSwitchProfile() {
    await clearLearnerSession()
    setFeedback({ kind: 'idle' })
    setScreen(await loadChoosingScreen())
  }

  if (screen.kind === 'loading') {
    return (
      <section className="home">
        <p>Loading your profile…</p>
      </section>
    )
  }

  if (screen.kind === 'selected') {
    return (
      <section className="home">
        <p>
          Current learner: <strong>{screen.learner.name}</strong>
        </p>
        <button type="button" onClick={handleSwitchProfile}>
          Switch profile
        </button>
      </section>
    )
  }

  return (
    <section className="home">
      <h2>Choose a profile</h2>
      {screen.learners.length > 0 && (
        <ul>
          {screen.learners.map((learner) => (
            <li key={learner.id}>
              <button type="button" onClick={() => handleSelect(learner.id)}>
                {learner.name}
              </button>
            </li>
          ))}
        </ul>
      )}

      <form onSubmit={handleCreate}>
        <label htmlFor="learner-name">New learner name</label>
        <input
          id="learner-name"
          value={newName}
          onChange={(event) => setNewName(event.target.value)}
        />
        <button type="submit" disabled={feedback.kind === 'submitting'}>
          Create profile
        </button>
      </form>

      {feedback.kind === 'error' && <p role="alert">{feedback.message}</p>}
    </section>
  )
}

export default Home
