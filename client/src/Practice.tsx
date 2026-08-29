import { useState } from 'react'
import type { PracticeSession } from './api/practiceSessions'

type PracticeProps = {
  session: PracticeSession
  onChooseAnotherCategory: () => void
}

/**
 * Renders the current (first unanswered) Question: a prompt and its
 * options — four translations plus "Don't know" — with temporary
 * selection only. Answer Submission and advancing to the next Question
 * are introduced in ticket 08 (grilled-spec.md sec. 3; ticket 07).
 */
const Practice = ({ session, onChooseAnotherCategory }: PracticeProps) => {
  const [selectedOptionId, setSelectedOptionId] = useState<number | undefined>(undefined)

  const currentQuestion = session.questions[session.answered_question_count]

  return (
    <section className="practice">
      <h2>Practice</h2>

      <p className="practice-progress">
        {session.answered_question_count}/{session.actual_question_count}
      </p>

      {currentQuestion ? (
        <div className="practice-question">
          <p className="practice-prompt">{currentQuestion.prompt_text}</p>
          <ul className="practice-options">
            {currentQuestion.options.map((option) => (
              <li key={option.id}>
                <button
                  type="button"
                  aria-pressed={selectedOptionId === option.id}
                  onClick={() => setSelectedOptionId(option.id)}
                >
                  {option.text}
                </button>
              </li>
            ))}
          </ul>
        </div>
      ) : (
        <p>Session complete.</p>
      )}

      <button type="button" onClick={onChooseAnotherCategory}>
        Choose another category
      </button>
    </section>
  )
}

export default Practice
