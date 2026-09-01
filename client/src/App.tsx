import { useCallback, useEffect, useState } from "react";
import type { ComponentProps } from "react";
import type { ApiError } from "./api/client";
import { listDueCards } from "./api/cards";
import type { PracticeCard } from "./api/cards";
import type { NewTerm, Term } from "./api/terms";
import { createTerm, deleteTerm, listTerms, patchTermNotes } from "./api/terms";
import { describeError } from "./errors";
import { ImportControl } from "./ImportControl";
import { Practice } from "./Practice";
import type { Optional, Result } from "./types/effects";
import "./App.css";

type VocabState =
  | { status: "loading" }
  | { status: "ready"; terms: Term[] }
  | { status: "failed"; error: ApiError };

type DueState =
  | { status: "loading" }
  | { status: "ready"; count: number }
  | { status: "failed"; error: ApiError };

type View = "vocab" | "practice";

export const App = () => {
  const [view, setView] = useState<View>("vocab");
  const [vocab, setVocab] = useState<VocabState>({ status: "loading" });
  const [due, setDue] = useState<DueState>({ status: "loading" });

  const refreshDue = useCallback(() => {
    void listDueCards(new Date().toISOString()).then((result) =>
      setDue(toDueState(result)),
    );
  }, []);

  const refreshVocab = useCallback(() => {
    void listTerms().then((result) => setVocab(toVocabState(result)));
  }, []);

  useEffect(() => {
    let live = true;
    void listTerms().then((result) => {
      if (live) setVocab(toVocabState(result));
    });
    return () => {
      live = false;
    };
  }, []);

  useEffect(refreshDue, [refreshDue]);

  const withTerms = (update: (terms: Term[]) => Term[]) =>
    setVocab((current) =>
      current.status === "ready"
        ? { status: "ready", terms: update(current.terms) }
        : current,
    );

  return (
    <main className="app">
      {/* Persistent so the due count is visible from wherever the app
          starts and while practising. */}
      <header className="topbar">
        <DueBadge state={due} />
      </header>
      {view === "vocab" ? (
        <>
          <div className="views">
            <h1>Vocab</h1>
            <button type="button" onClick={() => setView("practice")}>
              Practice
            </button>
          </div>
          <NewTermForm
            onAdded={(term) => withTerms((terms) => upsertTerm(terms, term))}
          />
          <ImportControl
            onImported={() => {
              refreshVocab();
              refreshDue();
            }}
          />
          <Vocab
            state={vocab}
            onNotesSaved={(term) => withTerms((terms) => upsertTerm(terms, term))}
            onDeleted={(id) => withTerms((terms) => removeTerm(terms, id))}
          />
        </>
      ) : (
        <Practice
          onCardPassed={() => setDue(decrementDue)}
          onExit={() => {
            setView("vocab");
            refreshDue();
          }}
        />
      )}
    </main>
  );
};

const DueBadge = ({ state }: { state: DueState }) => {
  switch (state.status) {
    case "loading":
      return (
        <span className="due-badge muted" aria-label="due count">
          … due
        </span>
      );
    case "failed":
      return (
        <span className="due-badge error" aria-label="due count">
          due count unavailable
        </span>
      );
    case "ready":
      return (
        <span className="due-badge" aria-label="due count">
          {state.count} due
        </span>
      );
  }
};

const Vocab = ({
  state,
  onNotesSaved,
  onDeleted,
}: {
  state: VocabState;
  onNotesSaved: (term: Term) => void;
  onDeleted: (id: string) => void;
}) => {
  switch (state.status) {
    case "loading":
      return <p className="muted">Loading…</p>;
    case "failed":
      return <p className="error">{describeError(state.error)}</p>;
    case "ready":
      return state.terms.length === 0 ? (
        <p className="muted">No terms yet. Add one above.</p>
      ) : (
        <table className="terms">
          <thead>
            <tr>
              <th>Foreign</th>
              <th>Pivot</th>
              <th>Lang</th>
              <th>Notes</th>
              <th aria-label="actions" />
            </tr>
          </thead>
          <tbody>
            {state.terms.map((term) => (
              <TermRow
                key={term.id}
                term={term}
                onNotesSaved={onNotesSaved}
                onDeleted={onDeleted}
              />
            ))}
          </tbody>
        </table>
      );
  }
};

const NewTermForm = ({ onAdded }: { onAdded: (term: Term) => void }) => {
  const [foreignLang, setForeignLang] = useState("");
  const [foreignText, setForeignText] = useState("");
  const [pivotText, setPivotText] = useState("");
  const [notes, setNotes] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<Optional<string>>(undefined);

  const draft: NewTerm = {
    foreign_lang: foreignLang.trim(),
    foreign_text: foreignText.trim(),
    pivot_text: pivotText.trim(),
    notes: blankToUndefined(notes),
  };
  const canSubmit = isCompleteDraft(draft) && !submitting;

  const handleSubmit: ComponentProps<"form">["onSubmit"] = (event) => {
    event.preventDefault();
    if (!canSubmit) return;
    setSubmitting(true);
    setError(undefined);
    void createTerm(draft).then((result) => {
      setSubmitting(false);
      if (result.ok) {
        onAdded(result.value);
        setForeignLang("");
        setForeignText("");
        setPivotText("");
        setNotes("");
      } else {
        setError(describeError(result.error));
      }
    });
  };

  return (
    <form className="new-term" onSubmit={handleSubmit}>
      <input
        aria-label="foreign text"
        placeholder="Foreign text"
        value={foreignText}
        onChange={(e) => setForeignText(e.target.value)}
      />
      <input
        aria-label="pivot text"
        placeholder="Pivot text"
        value={pivotText}
        onChange={(e) => setPivotText(e.target.value)}
      />
      <input
        aria-label="foreign lang"
        placeholder="Lang (e.g. es)"
        value={foreignLang}
        onChange={(e) => setForeignLang(e.target.value)}
      />
      <input
        aria-label="notes"
        placeholder="Notes (optional)"
        value={notes}
        onChange={(e) => setNotes(e.target.value)}
      />
      <button type="submit" disabled={!canSubmit}>
        {submitting ? "Adding…" : "Add term"}
      </button>
      {error !== undefined && <p className="error">{error}</p>}
    </form>
  );
};

const TermRow = ({
  term,
  onNotesSaved,
  onDeleted,
}: {
  term: Term;
  onNotesSaved: (term: Term) => void;
  onDeleted: (id: string) => void;
}) => {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<Optional<string>>(undefined);

  const startEdit = () => {
    setDraft(term.notes ?? "");
    setError(undefined);
    setEditing(true);
  };

  const save = () => {
    setBusy(true);
    setError(undefined);
    void patchTermNotes(term.id, blankToUndefined(draft)).then((result) => {
      setBusy(false);
      if (result.ok) {
        onNotesSaved(result.value);
        setEditing(false);
      } else {
        setError(describeError(result.error));
      }
    });
  };

  const remove = () => {
    if (!window.confirm(`Delete "${term.foreign_text}"? This cannot be undone.`))
      return;
    setBusy(true);
    setError(undefined);
    void deleteTerm(term.id).then((result) => {
      setBusy(false);
      if (result.ok) {
        onDeleted(term.id);
      } else {
        setError(describeError(result.error));
      }
    });
  };

  return (
    <tr>
      <td>{term.foreign_text}</td>
      <td>{term.pivot_text}</td>
      <td>{term.foreign_lang}</td>
      <td>
        {editing ? (
          <span className="notes-edit">
            <input
              aria-label={`notes for ${term.foreign_text}`}
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              disabled={busy}
            />
            <button type="button" onClick={save} disabled={busy}>
              Save
            </button>
            <button
              type="button"
              onClick={() => setEditing(false)}
              disabled={busy}
            >
              Cancel
            </button>
          </span>
        ) : (
          <button type="button" className="notes-display" onClick={startEdit}>
            {term.notes ?? <span className="muted">Add notes</span>}
          </button>
        )}
        {error !== undefined && <p className="error">{error}</p>}
      </td>
      <td>
        <button
          type="button"
          className="delete"
          onClick={remove}
          disabled={busy}
        >
          Delete
        </button>
      </td>
    </tr>
  );
};

// --- pure helpers --------------------------------------------------------

export const toVocabState = (result: Result<Term[], ApiError>): VocabState =>
  result.ok
    ? { status: "ready", terms: result.value }
    : { status: "failed", error: result.error };

/** The landing badge counts the Cards a `due_before=<now>` query returns. */
export const toDueState = (
  result: Result<PracticeCard[], ApiError>,
): DueState =>
  result.ok
    ? { status: "ready", count: result.value.length }
    : { status: "failed", error: result.error };

/** A passing Review promotes a Card out of the due set, so the badge ticks
 *  down; a failing one leaves it due, so the count is unchanged. */
export const decrementDue = (state: DueState): DueState =>
  state.status === "ready"
    ? { status: "ready", count: Math.max(0, state.count - 1) }
    : state;

/** Insert `term`, or replace the existing row with the same id. The
 *  server's add endpoint is idempotent on id, so a re-add returns an
 *  existing Term — this keeps the list free of duplicates either way. */
export const upsertTerm = (terms: Term[], term: Term): Term[] => {
  const known = terms.some((t) => t.id === term.id);
  return known
    ? terms.map((t) => (t.id === term.id ? term : t))
    : [...terms, term];
};

export const removeTerm = (terms: Term[], id: string): Term[] =>
  terms.filter((t) => t.id !== id);

export const isCompleteDraft = (draft: NewTerm): boolean =>
  draft.foreign_lang.length > 0 &&
  draft.foreign_text.length > 0 &&
  draft.pivot_text.length > 0;

const blankToUndefined = (value: string): Optional<string> => {
  const trimmed = value.trim();
  return trimmed.length === 0 ? undefined : trimmed;
};
