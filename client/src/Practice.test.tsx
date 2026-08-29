import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import Practice from './Practice'
import type { PracticeSession } from './api/practiceSessions'

const session: PracticeSession = {
  id: 5,
  learner_id: 1,
  category_id: 1,
  direction: 'source_to_target',
  status: 'active',
  requested_question_count: 10,
  actual_question_count: 2,
  answered_question_count: 0,
  correct_count: 0,
  started_at: '2026-08-12T00:00:00Z',
  completed_at: undefined,
  last_activity_at: '2026-08-12T00:00:00Z',
  created_at: '2026-08-12T00:00:00Z',
  updated_at: '2026-08-12T00:00:00Z',
  questions: [
    {
      id: 1,
      vocabulary_entry_id: 1,
      direction: 'source_to_target',
      ordinal: 1,
      prompt_text: 'manzana',
      options: [
        { id: 1, text: 'apple', is_dont_know: false },
        { id: 2, text: 'orange', is_dont_know: false },
        { id: 3, text: 'banana', is_dont_know: false },
        { id: 4, text: 'grape', is_dont_know: false },
        { id: 5, text: 'pear', is_dont_know: false },
        { id: 6, text: "Don't know", is_dont_know: true },
      ],
    },
    {
      id: 2,
      vocabulary_entry_id: 2,
      direction: 'source_to_target',
      ordinal: 2,
      prompt_text: 'naranja',
      options: [
        { id: 1, text: 'orange', is_dont_know: false },
        { id: 2, text: 'apple', is_dont_know: false },
        { id: 3, text: 'banana', is_dont_know: false },
        { id: 4, text: 'grape', is_dont_know: false },
        { id: 5, text: 'pear', is_dont_know: false },
        { id: 6, text: "Don't know", is_dont_know: true },
      ],
    },
  ],
}

describe('Practice', () => {
  // Practice screen: shows progress, the prompt, and every option — four
  // translations plus "Don't know" — without exposing the answer
  // (grilled-spec.md sec. 3; ticket 07).
  it('renders the prompt, progress, and options without exposing correctness', () => {
    render(<Practice session={session} onChooseAnotherCategory={vi.fn()} />)

    expect(screen.getByText('0/2')).toBeInTheDocument()
    expect(screen.getByText('manzana')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'apple' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: "Don't know" })).toBeInTheDocument()
    expect(
      screen.getAllByRole('button', { name: /apple|orange|banana|grape|pear|Don't know/ }),
    ).toHaveLength(6)
  })

  // Temporary selection before submission does not advance or score
  // anything (grilled-spec.md sec. 3; ticket 07 — submission itself lands
  // in ticket 08).
  it('allows a temporary selection without advancing the question', () => {
    render(<Practice session={session} onChooseAnotherCategory={vi.fn()} />)

    const appleButton = screen.getByRole('button', { name: 'apple' })
    fireEvent.click(appleButton)

    expect(appleButton).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByText('manzana')).toBeInTheDocument()
  })

  it('calls onChooseAnotherCategory when the learner backs out', () => {
    const onChooseAnotherCategory = vi.fn()
    render(<Practice session={session} onChooseAnotherCategory={onChooseAnotherCategory} />)

    fireEvent.click(screen.getByRole('button', { name: 'Choose another category' }))

    expect(onChooseAnotherCategory).toHaveBeenCalledTimes(1)
  })
})
