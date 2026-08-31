import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { PracticeCard } from "./api/cards";
import { ok } from "./types/effects";

vi.mock("./api/cards", () => ({
  listDueCards: vi.fn(),
  createReview: vi.fn(),
}));

const api = await import("./api/cards");
const { Practice, toPracticeState, advance, requeueFailed } = await import(
  "./Practice"
);

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

describe("pure helpers", () => {
  it("toPracticeState: an error result becomes the failed state", () => {
    const error = { kind: "network", detail: "offline" } as const;
    expect(toPracticeState({ ok: false, error })).toEqual({
      status: "failed",
      error,
    });
  });

  it("toPracticeState: an empty queue becomes the empty state", () => {
    expect(toPracticeState(ok([]))).toEqual({ status: "empty" });
  });

  it("toPracticeState: a non-empty queue starts a run on the first card", () => {
    const state = toPracticeState(ok([card({ id: "a" }), card({ id: "b" })]));
    expect(state).toMatchObject({
      status: "running",
      run: {
        current: { id: "a" },
        upcoming: [{ id: "b" }],
        revealed: false,
        tally: { passed: 0, failed: 0, seen: 0 },
      },
    });
  });

  it("requeueFailed puts the card at the back of the remaining queue", () => {
    expect(requeueFailed([card({ id: "b" }), card({ id: "c" })], card({ id: "a" }))
      .map((c) => c.id)).toEqual(["b", "c", "a"]);
    expect(requeueFailed([], card({ id: "a" })).map((c) => c.id)).toEqual(["a"]);
  });

  it("advance on a pass drops the card and tallies it", () => {
    const run = {
      current: card({ id: "a" }),
      upcoming: [card({ id: "b" })],
      revealed: true,
      tally: { passed: 0, failed: 0, seen: 0 },
    };
    expect(advance(run, "pass")).toMatchObject({
      status: "running",
      run: {
        current: { id: "b" },
        upcoming: [],
        revealed: false,
        tally: { passed: 1, failed: 0, seen: 1 },
      },
    });
  });

  it("advance on a fail sends the card to the back and moves on", () => {
    const run = {
      current: card({ id: "a" }),
      upcoming: [card({ id: "b" }), card({ id: "c" }), card({ id: "d" })],
      revealed: true,
      tally: { passed: 0, failed: 0, seen: 0 },
    };
    const next = advance(run, "fail");
    expect(next.status).toBe("running");
    if (next.status !== "running") return;
    expect(next.run.current.id).toBe("b");
    expect(next.run.upcoming.map((c) => c.id)).toEqual(["c", "d", "a"]);
    expect(next.run.tally).toEqual({ passed: 0, failed: 1, seen: 1 });
  });

  it("advance to an exhausted queue ends in the summary", () => {
    const run = {
      current: card({ id: "a" }),
      upcoming: [],
      revealed: true,
      tally: { passed: 2, failed: 1, seen: 3 },
    };
    expect(advance(run, "pass")).toEqual({
      status: "summary",
      tally: { passed: 3, failed: 1, seen: 4 },
    });
  });

  it("failing the last card shows it again rather than ending", () => {
    const run = {
      current: card({ id: "a" }),
      upcoming: [],
      revealed: true,
      tally: { passed: 0, failed: 0, seen: 0 },
    };
    const next = advance(run, "fail");
    expect(next).toMatchObject({ status: "running", run: { current: { id: "a" } } });
  });
});

describe("Practice screen", () => {
  it("reveals the answer and notes on demand, then grades and advances", async () => {
    vi.mocked(api.listDueCards).mockResolvedValue(
      ok([
        card({ id: "a", prompt: "gato", answer: "cat", notes: "el gato (m)" }),
        card({ id: "b", prompt: "perro", answer: "dog" }),
      ]),
    );
    vi.mocked(api.createReview).mockResolvedValue(ok(card({ id: "a", box: 2 })));
    const onCardPassed = vi.fn();
    const user = userEvent.setup();

    render(<Practice onCardPassed={onCardPassed} onExit={vi.fn()} />);

    expect(await screen.findByText("gato")).toBeInTheDocument();
    expect(screen.queryByText("cat")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Reveal answer" }));
    expect(screen.getByText("cat")).toBeInTheDocument();
    expect(screen.getByText("el gato (m)")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Pass" }));

    expect(api.createReview).toHaveBeenCalledWith("a", "pass");
    expect(onCardPassed).toHaveBeenCalledTimes(1);
    expect(await screen.findByText("perro")).toBeInTheDocument();
    expect(screen.queryByText("cat")).not.toBeInTheDocument();
  });

  it("re-offers a failed card before the sitting ends", async () => {
    vi.mocked(api.listDueCards).mockResolvedValue(
      ok([
        card({ id: "a", prompt: "uno", answer: "one" }),
        card({ id: "b", prompt: "dos", answer: "two" }),
      ]),
    );
    vi.mocked(api.createReview).mockImplementation((id) =>
      Promise.resolve(ok(card({ id }))),
    );
    const user = userEvent.setup();

    render(<Practice onCardPassed={vi.fn()} onExit={vi.fn()} />);

    // Fail "uno".
    await user.click(await screen.findByRole("button", { name: "Reveal answer" }));
    await user.click(screen.getByRole("button", { name: "Fail" }));

    // "dos" comes next; pass it.
    expect(await screen.findByText("dos")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Reveal answer" }));
    await user.click(screen.getByRole("button", { name: "Pass" }));

    // "uno" is back rather than a summary.
    expect(await screen.findByText("uno")).toBeInTheDocument();
    expect(screen.queryByText("Sitting complete")).not.toBeInTheDocument();
  });

  it("shows an end-of-run summary whose counts add up", async () => {
    vi.mocked(api.listDueCards).mockResolvedValue(
      ok([card({ id: "a", prompt: "uno" }), card({ id: "b", prompt: "dos" })]),
    );
    vi.mocked(api.createReview).mockImplementation((id) =>
      Promise.resolve(ok(card({ id }))),
    );
    const user = userEvent.setup();

    render(<Practice onCardPassed={vi.fn()} onExit={vi.fn()} />);

    for (let i = 0; i < 2; i++) {
      await user.click(await screen.findByRole("button", { name: "Reveal answer" }));
      await user.click(screen.getByRole("button", { name: "Pass" }));
    }

    expect(await screen.findByText("Sitting complete")).toBeInTheDocument();
    expect(screen.getByText("Passed 2 · Failed 0 · Seen 2")).toBeInTheDocument();
  });

  it("shows a clear empty state when nothing is due", async () => {
    vi.mocked(api.listDueCards).mockResolvedValue(ok([]));

    render(<Practice onCardPassed={vi.fn()} onExit={vi.fn()} />);

    expect(await screen.findByText("Nothing due right now.")).toBeInTheDocument();
  });

  it("surfaces a fetch failure through the normal error UI", async () => {
    vi.mocked(api.listDueCards).mockResolvedValue({
      ok: false,
      error: { kind: "network", detail: "offline" },
    });

    render(<Practice onCardPassed={vi.fn()} onExit={vi.fn()} />);

    expect(await screen.findByText(/Network error: offline/)).toBeInTheDocument();
  });

  it("surfaces a grading failure without advancing", async () => {
    vi.mocked(api.listDueCards).mockResolvedValue(
      ok([card({ id: "a", prompt: "uno" }), card({ id: "b", prompt: "dos" })]),
    );
    vi.mocked(api.createReview).mockResolvedValue({
      ok: false,
      error: { kind: "http", status: 500, message: "boom" },
    });
    const user = userEvent.setup();

    render(<Practice onCardPassed={vi.fn()} onExit={vi.fn()} />);

    await user.click(await screen.findByRole("button", { name: "Reveal answer" }));
    await user.click(screen.getByRole("button", { name: "Pass" }));

    expect(await screen.findByText(/Server error 500: boom/)).toBeInTheDocument();
    expect(screen.getByText("uno")).toBeInTheDocument();
  });
});
