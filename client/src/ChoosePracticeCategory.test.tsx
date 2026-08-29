import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ChoosePracticeCategory from './ChoosePracticeCategory'
import { listCategories } from './api/categories'
import { createPracticeSession } from './api/practiceSessions'
import { apiError } from './testUtils/envelopes'

vi.mock('./api/categories')
vi.mock('./api/practiceSessions')

const fruit = {
  id: 1,
  name: 'Fruit',
  created_at: '2026-08-12T00:00:00Z',
  updated_at: '2026-08-12T00:00:00Z',
}

const session = {
  id: 5,
  learner_id: 1,
  category_id: 1,
  direction: 'source_to_target' as const,
  status: 'active' as const,
  requested_question_count: 10,
  actual_question_count: 5,
  answered_question_count: 0,
  correct_count: 0,
  started_at: '2026-08-12T00:00:00Z',
  completed_at: undefined,
  last_activity_at: '2026-08-12T00:00:00Z',
  created_at: '2026-08-12T00:00:00Z',
  updated_at: '2026-08-12T00:00:00Z',
  questions: [],
}

describe('ChoosePracticeCategory', () => {
  beforeEach(() => {
    vi.resetAllMocks()
  })

  // Choose Practice Category screen: pick a Category, Direction, and
  // question count, then start a session (grilled-spec.md sec. 3; ticket
  // 07).
  it('starts a practice session with the chosen category, direction, and question count', async () => {
    vi.mocked(listCategories).mockResolvedValue({ ok: true, value: [fruit] })
    vi.mocked(createPracticeSession).mockResolvedValue({ ok: true, value: session })
    const onSessionStarted = vi.fn()

    render(<ChoosePracticeCategory onSessionStarted={onSessionStarted} />)

    await screen.findByText('Fruit')
    fireEvent.click(screen.getByLabelText('Target → Source'))
    fireEvent.change(screen.getByLabelText('Question count'), { target: { value: '15' } })
    fireEvent.click(screen.getByRole('button', { name: 'Start practice' }))

    await waitFor(() => expect(createPracticeSession).toHaveBeenCalledTimes(1))
    expect(createPracticeSession).toHaveBeenCalledWith(1, 'target_to_source', 15)
    expect(onSessionStarted).toHaveBeenCalledWith(session)
  })

  // Zero Eligible Entries (or any other rejection) surfaces the server's
  // message rather than starting a session (grilled-spec.md sec. 9; ticket
  // 07).
  it('shows the server error message when session creation is rejected', async () => {
    vi.mocked(listCategories).mockResolvedValue({ ok: true, value: [fruit] })
    vi.mocked(createPracticeSession).mockResolvedValue({
      ok: false,
      error: apiError(
        'No eligible vocabulary is available for this category and direction right now.',
      ),
    })

    render(<ChoosePracticeCategory onSessionStarted={vi.fn()} />)

    await screen.findByText('Fruit')
    fireEvent.click(screen.getByRole('button', { name: 'Start practice' }))

    expect(
      await screen.findByText(
        'No eligible vocabulary is available for this category and direction right now.',
      ),
    ).toBeInTheDocument()
  })

  // With no categories yet, the Learner is guided to add one before
  // starting practice (ticket 07).
  it('prompts to add a category when none exist', async () => {
    vi.mocked(listCategories).mockResolvedValue({ ok: true, value: [] })

    render(<ChoosePracticeCategory onSessionStarted={vi.fn()} />)

    expect(
      await screen.findByText(
        'Add a category and some vocabulary before starting a practice session.',
      ),
    ).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Start practice' })).not.toBeInTheDocument()
  })
})
