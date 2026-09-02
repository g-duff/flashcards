import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ApiError } from "./api/client";
import type { PracticeCard } from "./api/cards";
import type { Term } from "./api/terms";
import { ok } from "./types/effects";

vi.mock("./api/terms", () => ({
  listTerms: vi.fn(),
  createTerm: vi.fn(),
  patchTermNotes: vi.fn(),
  deleteTerm: vi.fn(),
  importTerms: vi.fn(),
}));

vi.mock("./api/cards", () => ({
  listDueCards: vi.fn(),
  createReview: vi.fn(),
}));

// Imported after the mocks are registered.
const api = await import("./api/terms");
const cardsApi = await import("./api/cards");
const {
  App,
  toVocabState,
  toDueState,
  decrementDue,
  upsertTerm,
  removeTerm,
  isCompleteDraft,
} = await import("./App");

const term = (over: Partial<Term> = {}): Term => ({
  id: "id-1",
  foreign_lang: "es",
  foreign_text: "perro",
  pivot_text: "dog",
  notes: undefined,
  created_at: "2026-01-01T00:00:00Z",
  ...over,
});

const card = (over: Partial<PracticeCard> = {}): PracticeCard => ({
  id: "card-1",
  term_id: "term-1",
  prompt_side: "foreign",
  prompt: "gato",
  answer: "cat",
  notes: undefined,
  due_at: "2026-01-01T00:00:00Z",
  box: 1,
  ...over,
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

// The Vocab-screen tests below don't exercise Practice, but App still
// queries the due count on mount for the badge.
vi.mocked(cardsApi.listDueCards).mockResolvedValue(ok([]));

describe("pure helpers", () => {
  it("maps a failed result to the failed view state", () => {
    const error: ApiError = { kind: "http", status: 500, message: "boom" };
    expect(toVocabState({ ok: false, error })).toEqual({
      status: "failed",
      error,
    });
  });

  it("upsertTerm appends an unknown term and replaces a known one", () => {
    const a = term({ id: "a" });
    const b = term({ id: "b", foreign_text: "gato" });
    expect(upsertTerm([a], b)).toEqual([a, b]);

    const bEdited = term({ id: "b", foreign_text: "gato", notes: "el gato" });
    expect(upsertTerm([a, b], bEdited)).toEqual([a, bEdited]);
    expect(upsertTerm([a, b], bEdited)).toHaveLength(2);
  });

  it("removeTerm drops the matching id", () => {
    const a = term({ id: "a" });
    const b = term({ id: "b" });
    expect(removeTerm([a, b], "a")).toEqual([b]);
  });

  it("isCompleteDraft requires all three text fields", () => {
    expect(
      isCompleteDraft({ foreign_lang: "es", foreign_text: "perro", pivot_text: "dog" }),
    ).toBe(true);
    expect(
      isCompleteDraft({ foreign_lang: "es", foreign_text: "", pivot_text: "dog" }),
    ).toBe(false);
  });

  it("toDueState reads the count from the returned list, or carries the error", () => {
    expect(toDueState(ok([card(), card()]))).toEqual({
      status: "ready",
      count: 2,
    });
    const error: ApiError = { kind: "network", detail: "x" };
    expect(toDueState({ ok: false, error })).toEqual({ status: "failed", error });
  });

  it("decrementDue ticks a ready count down but never below zero", () => {
    expect(decrementDue({ status: "ready", count: 3 })).toEqual({
      status: "ready",
      count: 2,
    });
    expect(decrementDue({ status: "ready", count: 0 })).toEqual({
      status: "ready",
      count: 0,
    });
    expect(decrementDue({ status: "loading" })).toEqual({ status: "loading" });
  });
});

describe("Vocab screen", () => {
  it("adds a term through the inline form once every text field is set", async () => {
    vi.mocked(api.listTerms).mockResolvedValue(ok([]));
    vi.mocked(api.createTerm).mockResolvedValue(ok(term({ notes: "el perro (m)" })));
    const user = userEvent.setup();

    render(<App />);
    await screen.findByText("No terms yet. Add one above.");

    const addButton = screen.getByRole("button", { name: "Add term" });
    expect(addButton).toBeDisabled();

    await user.type(screen.getByLabelText("foreign text"), "perro");
    await user.type(screen.getByLabelText("pivot text"), "dog");
    await user.type(screen.getByLabelText("foreign lang"), "es");
    expect(addButton).toBeEnabled();

    await user.click(addButton);

    expect(api.createTerm).toHaveBeenCalledWith({
      foreign_lang: "es",
      foreign_text: "perro",
      pivot_text: "dog",
      notes: undefined,
    });
    expect(await screen.findByText("perro")).toBeInTheDocument();
    expect(screen.getByText("el perro (m)")).toBeInTheDocument();
  });

  it("edits notes inline and shows the saved value", async () => {
    vi.mocked(api.listTerms).mockResolvedValue(ok([term()]));
    vi.mocked(api.patchTermNotes).mockResolvedValue(ok(term({ notes: "el perro" })));
    const user = userEvent.setup();

    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Add notes" }));
    await user.type(screen.getByLabelText("notes for perro"), "el perro");
    await user.click(screen.getByRole("button", { name: "Save" }));

    expect(api.patchTermNotes).toHaveBeenCalledWith("id-1", "el perro");
    expect(await screen.findByText("el perro")).toBeInTheDocument();
  });

  it("deletes a term only after the confirm is accepted", async () => {
    vi.mocked(api.listTerms).mockResolvedValue(ok([term()]));
    vi.mocked(api.deleteTerm).mockResolvedValue(ok({ deleted: "id-1" }));
    const user = userEvent.setup();
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);

    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Delete" }));
    expect(api.deleteTerm).not.toHaveBeenCalled();

    confirm.mockReturnValue(true);
    await user.click(screen.getByRole("button", { name: "Delete" }));

    expect(api.deleteTerm).toHaveBeenCalledWith("id-1");
    await waitFor(() =>
      expect(screen.getByText("No terms yet. Add one above.")).toBeInTheDocument(),
    );
    confirm.mockRestore();
  });

  it("imports a file and refreshes the term table from the server", async () => {
    vi.mocked(api.listTerms)
      .mockResolvedValueOnce(ok([]))
      .mockResolvedValue(ok([term({ foreign_text: "perro" })]));
    vi.mocked(api.importTerms).mockResolvedValue(ok({ imported: 1, skipped: 0 }));
    const user = userEvent.setup();

    render(<App />);
    await screen.findByText("No terms yet. Add one above.");

    await user.upload(
      screen.getByLabelText("vocab file"),
      new File(["es,perro,dog,el perro (m)"], "vocab.csv", { type: "text/csv" }),
    );
    await user.click(await screen.findByRole("button", { name: "Import" }));

    expect(api.importTerms).toHaveBeenCalledWith([
      {
        foreign_lang: "es",
        foreign_text: "perro",
        pivot_text: "dog",
        notes: "el perro (m)",
      },
    ]);
    expect(await screen.findByText("Imported 1, skipped 0.")).toBeInTheDocument();
    expect(await screen.findByText("perro")).toBeInTheDocument();
  });
});

describe("due badge and Practice navigation", () => {
  it("shows the due count on the landing view from a due_before query", async () => {
    vi.mocked(api.listTerms).mockResolvedValue(ok([]));
    vi.mocked(cardsApi.listDueCards).mockResolvedValue(
      ok([card({ id: "c1" }), card({ id: "c2" }), card({ id: "c3" })]),
    );

    render(<App />);

    expect(await screen.findByLabelText("due count")).toHaveTextContent("3 due");
  });

  it("ticks the badge down as Cards are passed in Practice", async () => {
    vi.mocked(api.listTerms).mockResolvedValue(ok([]));
    vi.mocked(cardsApi.listDueCards).mockResolvedValue(
      ok([card({ id: "c1", prompt: "uno" }), card({ id: "c2", prompt: "dos" })]),
    );
    vi.mocked(cardsApi.createReview).mockImplementation((id) =>
      Promise.resolve(ok(card({ id }))),
    );
    const user = userEvent.setup();

    render(<App />);

    expect(await screen.findByLabelText("due count")).toHaveTextContent("2 due");

    await user.click(screen.getByRole("button", { name: "Practice" }));
    await user.click(await screen.findByRole("button", { name: "Reveal answer" }));
    await user.click(screen.getByRole("button", { name: "Pass" }));

    await waitFor(() =>
      expect(screen.getByLabelText("due count")).toHaveTextContent("1 due"),
    );
  });
});
