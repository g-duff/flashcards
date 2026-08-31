import { describe, expect, it } from "vitest";
import { describeError } from "./errors";

describe("describeError", () => {
  it("is exhaustive over the ApiError kinds", () => {
    expect(describeError({ kind: "network", detail: "x" })).toMatch(/network/i);
    expect(describeError({ kind: "http", status: 404, message: "nope" })).toMatch(
      /404/,
    );
    expect(describeError({ kind: "malformed", detail: "x" })).toMatch(
      /bad response/i,
    );
  });
});
