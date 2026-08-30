import { useEffect, useState } from "react";
import type { ComponentProps } from "react";
import type { ApiError, Card } from "./api";
import { createCard, listCards } from "./api";
import type { Optional, Result } from "./types/effects";
import "./App.css";

type DeckState =
  | { status: "loading" }
  | { status: "ready"; cards: Card[] }
  | { status: "failed"; error: ApiError };

export const App = () => {
  const [deck, setDeck] = useState<DeckState>({ status: "loading" });

  useEffect(() => {
    let live = true;
    void listCards().then((result) => {
      if (!live) return;
      setDeck(toDeckState(result));
    });
    return () => {
      live = false;
    };
  }, []);

  const handleCreated = (card: Card) => {
    setDeck((current) =>
      current.status === "ready"
        ? { status: "ready", cards: [...current.cards, card] }
        : current,
    );
  };

  return (
    <main className="app">
      <h1>Flashcards</h1>
      <NewCardForm onCreated={handleCreated} />
      <Deck state={deck} />
    </main>
  );
};

const Deck = ({ state }: { state: DeckState }) => {
  switch (state.status) {
    case "loading":
      return <p className="muted">Loading…</p>;
    case "failed":
      return <p className="error">{describeError(state.error)}</p>;
    case "ready":
      return state.cards.length === 0 ? (
        <p className="muted">No cards yet. Add one above.</p>
      ) : (
        <ul className="cards">
          {state.cards.map((card) => (
            <li key={card.id} className="card">
              <span className="front">{card.front}</span>
              <span className="back">{card.back}</span>
            </li>
          ))}
        </ul>
      );
  }
};

const NewCardForm = ({ onCreated }: { onCreated: (card: Card) => void }) => {
  const [front, setFront] = useState("");
  const [back, setBack] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<Optional<string>>(undefined);

  const canSubmit = front.trim().length > 0 && back.trim().length > 0 && !submitting;

  // Param type is inferred from the `onSubmit` prop — React 19's types
  // deprecate the standalone `FormEvent` alias.
  const handleSubmit: ComponentProps<"form">["onSubmit"] = (event) => {
    event.preventDefault();
    if (!canSubmit) return;
    setSubmitting(true);
    setError(undefined);
    void createCard({ front, back }).then((result) => {
      setSubmitting(false);
      if (result.ok) {
        onCreated(result.value);
        setFront("");
        setBack("");
      } else {
        setError(describeError(result.error));
      }
    });
  };

  return (
    <form className="new-card" onSubmit={handleSubmit}>
      <input
        aria-label="front"
        placeholder="Front"
        value={front}
        onChange={(e) => setFront(e.target.value)}
      />
      <input
        aria-label="back"
        placeholder="Back"
        value={back}
        onChange={(e) => setBack(e.target.value)}
      />
      <button type="submit" disabled={!canSubmit}>
        {submitting ? "Adding…" : "Add card"}
      </button>
      {error !== undefined && <p className="error">{error}</p>}
    </form>
  );
};

// --- pure helpers --------------------------------------------------------

const toDeckState = (result: Result<Card[], ApiError>): DeckState =>
  result.ok
    ? { status: "ready", cards: result.value }
    : { status: "failed", error: result.error };

const describeError = (error: ApiError): string => {
  switch (error.kind) {
    case "network":
      return `Network error: ${error.detail}`;
    case "http":
      return `Server error ${error.status}: ${error.message}`;
    case "malformed":
      return `Bad response from server: ${error.detail}`;
  }
};
