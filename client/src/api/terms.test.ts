import { afterEach, describe, expect, it, vi } from "vitest";
import { createTerm, deleteTerm, listTerms, patchTermNotes } from "./terms";

const jsonResponse = (body: unknown, status = 200): Response =>
  new Response(JSON.stringify(body), { status });

const stubFetch = (response: Response) =>
  vi.spyOn(globalThis, "fetch").mockResolvedValue(response);

afterEach(() => {
  vi.restoreAllMocks();
});

describe("terms API", () => {
  it("lists terms, normalising server nulls to undefined", async () => {
    const fetchMock = stubFetch(
      jsonResponse([
        {
          id: "id-1",
          foreign_lang: "es",
          foreign_text: "perro",
          pivot_text: "dog",
          notes: null,
          created_at: "2026-01-01T00:00:00Z",
        },
      ]),
    );

    const result = await listTerms();

    expect(fetchMock).toHaveBeenCalledWith("/flashcards/api/terms", {
      headers: { "content-type": "application/json" },
    });
    expect(result).toEqual({
      ok: true,
      value: [
        {
          id: "id-1",
          foreign_lang: "es",
          foreign_text: "perro",
          pivot_text: "dog",
          notes: undefined,
          created_at: "2026-01-01T00:00:00Z",
        },
      ],
    });
  });

  it("creates a term with a POST body", async () => {
    const created = {
      id: "id-1",
      foreign_lang: "es",
      foreign_text: "perro",
      pivot_text: "dog",
      notes: "el perro (m)",
      created_at: "2026-01-01T00:00:00Z",
    };
    const fetchMock = stubFetch(jsonResponse(created, 201));

    const result = await createTerm({
      foreign_lang: "es",
      foreign_text: "perro",
      pivot_text: "dog",
    });

    expect(fetchMock).toHaveBeenCalledWith("/flashcards/api/terms", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        foreign_lang: "es",
        foreign_text: "perro",
        pivot_text: "dog",
      }),
    });
    expect(result).toEqual({ ok: true, value: created });
  });

  it("sends notes: null when clearing, and encodes the id", async () => {
    const fetchMock = stubFetch(jsonResponse({ id: "a/b" }, 200));

    await patchTermNotes("a/b", undefined);

    expect(fetchMock).toHaveBeenCalledWith("/flashcards/api/terms/a%2Fb", {
      method: "PATCH",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ notes: null }),
    });
  });

  it("deletes a term and returns the deleted id", async () => {
    const fetchMock = stubFetch(jsonResponse({ deleted: "id-1" }, 200));

    const result = await deleteTerm("id-1");

    expect(fetchMock).toHaveBeenCalledWith("/flashcards/api/terms/id-1", {
      method: "DELETE",
      headers: { "content-type": "application/json" },
    });
    expect(result).toEqual({ ok: true, value: { deleted: "id-1" } });
  });

  it("turns a non-OK response into an http ApiError using the error body", async () => {
    stubFetch(jsonResponse({ error: "term not found" }, 404));

    const result = await deleteTerm("missing");

    expect(result).toEqual({
      ok: false,
      error: { kind: "http", status: 404, message: "term not found" },
    });
  });

  it("turns a thrown fetch into a network ApiError", async () => {
    vi.spyOn(globalThis, "fetch").mockRejectedValue(new Error("offline"));

    const result = await listTerms();

    expect(result).toEqual({
      ok: false,
      error: { kind: "network", detail: "Error: offline" },
    });
  });
});
