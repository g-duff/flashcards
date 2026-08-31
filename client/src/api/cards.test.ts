import { afterEach, describe, expect, it, vi } from "vitest";
import { createReview, listDueCards } from "./cards";

const jsonResponse = (body: unknown, status = 200): Response =>
  new Response(JSON.stringify(body), { status });

const stubFetch = (response: Response) =>
  vi.spyOn(globalThis, "fetch").mockResolvedValue(response);

const cardBody = (over: Record<string, unknown> = {}) => ({
  id: "card-1",
  term_id: "term-1",
  prompt_side: "foreign",
  prompt: "gato",
  answer: "cat",
  notes: null,
  due_at: "2026-01-01T00:00:00Z",
  box: 1,
  ...over,
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("cards API", () => {
  it("lists due cards with due_before and limit, normalising server nulls", async () => {
    const fetchMock = stubFetch(jsonResponse([cardBody()]));

    const result = await listDueCards("2026-08-31T12:00:00.000Z", 20);

    expect(fetchMock).toHaveBeenCalledWith(
      "/flashcards/api/cards?due_before=2026-08-31T12%3A00%3A00.000Z&limit=20",
      { headers: { "content-type": "application/json" } },
    );
    expect(result).toEqual({
      ok: true,
      value: [
        {
          id: "card-1",
          term_id: "term-1",
          prompt_side: "foreign",
          prompt: "gato",
          answer: "cat",
          notes: undefined,
          due_at: "2026-01-01T00:00:00Z",
          box: 1,
        },
      ],
    });
  });

  it("omits the limit param when no limit is given (badge count query)", async () => {
    const fetchMock = stubFetch(jsonResponse([]));

    await listDueCards("2026-08-31T12:00:00.000Z");

    expect(fetchMock).toHaveBeenCalledWith(
      "/flashcards/api/cards?due_before=2026-08-31T12%3A00%3A00.000Z",
      { headers: { "content-type": "application/json" } },
    );
  });

  it("posts a rating to a card's reviews and returns the rescheduled card", async () => {
    const fetchMock = stubFetch(jsonResponse(cardBody({ box: 2, notes: "el gato" })));

    const result = await createReview("card-1", "pass");

    expect(fetchMock).toHaveBeenCalledWith("/flashcards/api/cards/card-1/reviews", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ rating: "pass" }),
    });
    expect(result).toEqual({ ok: true, value: cardBody({ box: 2, notes: "el gato" }) });
  });

  it("encodes the card id in the reviews path", async () => {
    const fetchMock = stubFetch(jsonResponse(cardBody()));

    await createReview("a/b", "fail");

    expect(fetchMock).toHaveBeenCalledWith(
      "/flashcards/api/cards/a%2Fb/reviews",
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("surfaces a non-OK response as an http ApiError", async () => {
    stubFetch(jsonResponse({ error: "no card with that id" }, 404));

    const result = await createReview("missing", "pass");

    expect(result).toEqual({
      ok: false,
      error: { kind: "http", status: 404, message: "no card with that id" },
    });
  });

  it("surfaces a thrown fetch as a network ApiError", async () => {
    vi.spyOn(globalThis, "fetch").mockRejectedValue(new Error("offline"));

    const result = await listDueCards("2026-08-31T12:00:00.000Z", 20);

    expect(result).toEqual({
      ok: false,
      error: { kind: "network", detail: "Error: offline" },
    });
  });
});
