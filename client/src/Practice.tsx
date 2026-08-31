import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import type { PracticeCard, Rating } from "./api/cards";
import { createReview, listDueCards } from "./api/cards";
import type { ApiError } from "./api/client";
import { describeError } from "./errors";
import type { Optional, Result } from "./types/effects";

// The practice queue is pulled once at the start of a sitting.
const QUEUE_LIMIT = 20;


/** Running totals for one sitting. `seen` counts graded attempts, so a
 *  Card failed then later passed contributes to all three. */
export type Tally = { passed: number; failed: number; seen: number };

/** The live position in a sitting: the Card on screen, whether its answer
 *  is showing, the Cards still to come, and the totals so far. */
export type Run = {
  current: PracticeCard;
  upcoming: PracticeCard[];
  revealed: boolean;
  tally: Tally;
};

export type PracticeState =
  | { status: "loading" }
  | { status: "failed"; error: ApiError }
  | { status: "empty" }
  | { status: "running"; run: Run }
  | { status: "summary"; tally: Tally };

const EMPTY_TALLY: Tally = { passed: 0, failed: 0, seen: 0 };

// --- pure helpers ------------------------------------------------------

/** Fold the opening `GET /cards` result into a view state: an error, an
 *  empty sitting, or a run positioned on the first Card. */
export const toPracticeState = (
  result: Result<PracticeCard[], ApiError>,
): PracticeState => {
  if (!result.ok) return { status: "failed", error: result.error };
  const [current, ...upcoming] = result.value;
  if (current === undefined) return { status: "empty" };
  return {
    status: "running",
    run: { current, upcoming, revealed: false, tally: EMPTY_TALLY },
  };
};

/** Put a failed Card at the back of the remaining queue, so it comes
 *  round again this sitting but is never the very next Card — unless it
 *  was the only Card left, in which case it has to repeat. */
export const requeueFailed = (
  upcoming: PracticeCard[],
  card: PracticeCard,
): PracticeCard[] => [...upcoming, card];

const tallied = (tally: Tally, rating: Rating): Tally => ({
  passed: tally.passed + (rating === "pass" ? 1 : 0),
  failed: tally.failed + (rating === "fail" ? 1 : 0),
  seen: tally.seen + 1,
});

/** Apply a grade to the current Card: record it in the tally, re-queue
 *  the Card if it was failed, then move to the next Card — or to the
 *  end-of-run summary when nothing is left. */
export const advance = (
  run: Run,
  rating: Rating,
): { status: "running"; run: Run } | { status: "summary"; tally: Tally } => {
  const tally = tallied(run.tally, rating);
  const queue =
    rating === "fail"
      ? requeueFailed(run.upcoming, run.current)
      : run.upcoming;
  const [current, ...upcoming] = queue;
  return current === undefined
    ? { status: "summary", tally }
    : {
        status: "running",
        run: { current, upcoming, revealed: false, tally },
      };
};

// --- component -------------------------------------------------------

type PracticeProps = {
  /** Called after each Card the learner passes, so the landing badge can
   *  tick down without a refetch. */
  onCardPassed: () => void;
  /** Leave Practice and return to the landing view. */
  onExit: () => void;
};

export const Practice = ({ onCardPassed, onExit }: PracticeProps) => {
  const [state, setState] = useState<PracticeState>({ status: "loading" });
  const [grading, setGrading] = useState(false);
  const [gradeError, setGradeError] = useState<Optional<string>>(undefined);

  useEffect(() => {
    let live = true;
    void listDueCards(new Date().toISOString(), QUEUE_LIMIT).then((result) => {
      if (live) setState(toPracticeState(result));
    });
    return () => {
      live = false;
    };
  }, []);

  if (state.status === "loading")
    return (
      <PracticeShell onExit={onExit}>
        <p className="muted">Loading…</p>
      </PracticeShell>
    );

  if (state.status === "failed")
    return (
      <PracticeShell onExit={onExit}>
        <p className="error">{describeError(state.error)}</p>
      </PracticeShell>
    );

  if (state.status === "empty")
    return (
      <PracticeShell onExit={onExit}>
        <p className="muted">Nothing due right now.</p>
      </PracticeShell>
    );

  if (state.status === "summary") {
    const { passed, failed, seen } = state.tally;
    return (
      <PracticeShell onExit={onExit}>
        <div className="summary">
          <h3>Sitting complete</h3>
          <p>
            Passed {passed} · Failed {failed} · Seen {seen}
          </p>
          <button type="button" onClick={onExit}>
            Back to start
          </button>
        </div>
      </PracticeShell>
    );
  }

  const { run } = state;

  const grade = (rating: Rating) => {
    if (grading) return;
    setGrading(true);
    setGradeError(undefined);
    const card = run.current;
    void createReview(card.id, rating).then((result) => {
      setGrading(false);
      if (!result.ok) {
        setGradeError(describeError(result.error));
        return;
      }
      if (rating === "pass") onCardPassed();
      setState(advance(run, rating));
    });
  };

  return (
    <PracticeShell onExit={onExit}>
      <div className="practice-card">
        <p className="prompt">{run.current.prompt}</p>
        {run.revealed ? (
          <>
            <p className="answer">{run.current.answer}</p>
            {run.current.notes !== undefined && (
              <p className="card-notes muted">{run.current.notes}</p>
            )}
            <div className="grade">
              <button
                type="button"
                className="pass"
                onClick={() => grade("pass")}
                disabled={grading}
              >
                Pass
              </button>
              <button
                type="button"
                className="fail"
                onClick={() => grade("fail")}
                disabled={grading}
              >
                Fail
              </button>
            </div>
          </>
        ) : (
          <button
            type="button"
            onClick={() =>
              setState({ status: "running", run: { ...run, revealed: true } })
            }
          >
            Reveal answer
          </button>
        )}
        {gradeError !== undefined && <p className="error">{gradeError}</p>}
      </div>
    </PracticeShell>
  );
};

const PracticeShell = ({
  onExit,
  children,
}: {
  onExit: () => void;
  children: ReactNode;
}) => (
  <section className="practice">
    <div className="practice-head">
      <h2>Practice</h2>
      <button type="button" className="link" onClick={onExit}>
        Leave
      </button>
    </div>
    {children}
  </section>
);
