import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ApiError } from "./api/client";
import type { Term } from "./api/terms";
import { ok } from "./types/effects";

vi.mock("./api/terms", () => ({
  listTerms: vi.fn(),
  createTerm: vi.fn(),
  patchTermNotes: vi.fn(),
  deleteTerm: vi.fn(),
}));

// Imported after the mock is registered.
const api = await import("./api/terms");
const {
  App,
  toVocabState,
  upsertTerm,
  removeTerm,
  isCompleteDraft,
  describeError,
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

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

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

  it("describeError is exhaustive over ApiError kinds", () => {
    expect(describeError({ kind: "network", detail: "x" })).toMatch(/network/i);
    expect(describeError({ kind: "http", status: 404, message: "nope" })).toMatch(
      /404/,
    );
    expect(describeError({ kind: "malformed", detail: "x" })).toMatch(/bad response/i);
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
});
