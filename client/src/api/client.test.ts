import { afterEach, describe, expect, it, vi } from "vitest";
import { apiDelete, apiGet, apiPatch, apiPost } from "./client";

const textResponse = (
  body: string | null,
  init: ResponseInit = { status: 200 },
): Response => new Response(body, init);

const jsonResponse = (body: unknown, status = 200): Response =>
  new Response(JSON.stringify(body), { status });

const stubFetch = (response: Response) =>
  vi.spyOn(globalThis, "fetch").mockResolvedValue(response);

afterEach(() => {
  vi.restoreAllMocks();
});

describe("api/client verb helpers", () => {
  describe("request wiring", () => {
    it("apiGet prefixes the path and sends the JSON content-type, no method", async () => {
      const fetchMock = stubFetch(jsonResponse({ ok: 1 }));

      await apiGet("/terms");

      expect(fetchMock).toHaveBeenCalledWith("/flashcards/api/terms", {
        headers: { "content-type": "application/json" },
      });
    });

    it("apiPost sends method POST and a JSON-stringified body", async () => {
      const fetchMock = stubFetch(jsonResponse({}, 201));

      await apiPost("/terms", { foreign_text: "perro" });

      expect(fetchMock).toHaveBeenCalledWith("/flashcards/api/terms", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ foreign_text: "perro" }),
      });
    });

    it("apiPatch sends method PATCH and a JSON-stringified body", async () => {
      const fetchMock = stubFetch(jsonResponse({}));

      await apiPatch("/terms/id-1", { notes: null });

      expect(fetchMock).toHaveBeenCalledWith("/flashcards/api/terms/id-1", {
        method: "PATCH",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ notes: null }),
      });
    });

    it("apiDelete sends method DELETE and no body", async () => {
      const fetchMock = stubFetch(jsonResponse({ deleted: "id-1" }));

      await apiDelete("/terms/id-1");

      expect(fetchMock).toHaveBeenCalledWith("/flashcards/api/terms/id-1", {
        method: "DELETE",
        headers: { "content-type": "application/json" },
      });
    });
  });

  describe("successful responses", () => {
    it("parses a JSON body into ok(value)", async () => {
      stubFetch(jsonResponse({ id: "id-1", notes: "hi" }));

      const result = await apiGet("/terms/id-1");

      expect(result).toEqual({ ok: true, value: { id: "id-1", notes: "hi" } });
    });

    it("maps an empty body to ok(undefined)", async () => {
      stubFetch(textResponse("", { status: 200 }));

      const result = await apiGet("/terms");

      expect(result).toEqual({ ok: true, value: undefined });
    });

    it("maps a 204 No Content to ok(undefined)", async () => {
      stubFetch(textResponse(null, { status: 204 }));

      const result = await apiDelete("/terms/id-1");

      expect(result).toEqual({ ok: true, value: undefined });
    });

    it("normalises a top-level JSON null to undefined", async () => {
      stubFetch(textResponse("null"));

      const result = await apiGet("/terms/id-1");

      expect(result).toEqual({ ok: true, value: undefined });
    });

    it("normalises nested nulls in objects and arrays to undefined", async () => {
      stubFetch(
        jsonResponse([
          { id: "id-1", notes: null, tags: ["a", null] },
          { id: "id-2", notes: "keep", meta: { source: null } },
        ]),
      );

      const result = await apiGet("/terms");

      expect(result).toEqual({
        ok: true,
        value: [
          { id: "id-1", notes: undefined, tags: ["a", undefined] },
          { id: "id-2", notes: "keep", meta: { source: undefined } },
        ],
      });
    });

    it("passes primitive JSON bodies through untouched", async () => {
      stubFetch(jsonResponse(42));

      const result = await apiGet("/count");

      expect(result).toEqual({ ok: true, value: 42 });
    });

    it("returns a malformed ApiError when an OK body is not valid JSON", async () => {
      stubFetch(textResponse("{not json", { status: 200 }));

      const result = await apiGet("/terms");

      expect(result.ok).toBe(false);
      expect(result).toEqual({
        ok: false,
        error: { kind: "malformed", detail: expect.stringMatching(/SyntaxError/) },
      });
    });
  });

  describe("HTTP error responses", () => {
    it("uses the { error } convention for the message", async () => {
      stubFetch(jsonResponse({ error: "term not found" }, 404));

      const result = await apiGet("/terms/missing");

      expect(result).toEqual({
        ok: false,
        error: { kind: "http", status: 404, message: "term not found" },
      });
    });

    it("falls back to statusText when the error body has no string 'error' key", async () => {
      stubFetch(
        textResponse(JSON.stringify({ error: 123, detail: "nope" }), {
          status: 400,
          statusText: "Bad Request",
        }),
      );

      const result = await apiPost("/terms", {});

      expect(result).toEqual({
        ok: false,
        error: { kind: "http", status: 400, message: "Bad Request" },
      });
    });

    it("falls back to statusText when the error body is not JSON", async () => {
      stubFetch(
        textResponse("upstream exploded", {
          status: 502,
          statusText: "Bad Gateway",
        }),
      );

      const result = await apiGet("/terms");

      expect(result).toEqual({
        ok: false,
        error: { kind: "http", status: 502, message: "Bad Gateway" },
      });
    });

    it("falls back to statusText when the error body is JSON null", async () => {
      stubFetch(
        textResponse("null", { status: 500, statusText: "Internal Server Error" }),
      );

      const result = await apiGet("/terms");

      expect(result).toEqual({
        ok: false,
        error: { kind: "http", status: 500, message: "Internal Server Error" },
      });
    });

    it("falls back to statusText when the error body is empty", async () => {
      stubFetch(textResponse("", { status: 403, statusText: "Forbidden" }));

      const result = await apiDelete("/terms/id-1");

      expect(result).toEqual({
        ok: false,
        error: { kind: "http", status: 403, message: "Forbidden" },
      });
    });

    it("does not treat a non-OK response as malformed even if the body is junk", async () => {
      stubFetch(
        textResponse("<html>500</html>", {
          status: 500,
          statusText: "Internal Server Error",
        }),
      );

      const result = await apiGet("/terms");

      expect(result).toEqual({
        ok: false,
        error: { kind: "http", status: 500, message: "Internal Server Error" },
      });
    });
  });

  describe("network failures", () => {
    it("turns a thrown fetch into a network ApiError", async () => {
      vi.spyOn(globalThis, "fetch").mockRejectedValue(new Error("offline"));

      const result = await apiGet("/terms");

      expect(result).toEqual({
        ok: false,
        error: { kind: "network", detail: "Error: offline" },
      });
    });

    it("stringifies a non-Error rejection reason", async () => {
      vi.spyOn(globalThis, "fetch").mockRejectedValue("boom");

      const result = await apiPost("/terms", {});

      expect(result).toEqual({
        ok: false,
        error: { kind: "network", detail: "boom" },
      });
    });
  });
});
