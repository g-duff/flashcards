import { describe, expect, it } from "vitest";
import { canImport, parseVocab, termsToImport } from "./import";

describe("parseVocab", () => {
  it("parses rows with the default comma delimiter", () => {
    const text = "es,perro,dog,el perro (m)\nes,gato,cat,";
    const result = parseVocab(text, ",");

    expect(result.errors).toEqual([]);
    expect(result.rows).toEqual([
      {
        line: 1,
        term: {
          foreign_lang: "es",
          foreign_text: "perro",
          pivot_text: "dog",
          notes: "el perro (m)",
        },
      },
      {
        line: 2,
        term: {
          foreign_lang: "es",
          foreign_text: "gato",
          pivot_text: "cat",
          notes: undefined,
        },
      },
    ]);
  });

  it("parses rows with a custom delimiter", () => {
    const result = parseVocab("es;perro;dog;el perro (m)", ";");

    expect(result.errors).toEqual([]);
    expect(result.rows).toEqual([
      {
        line: 1,
        term: {
          foreign_lang: "es",
          foreign_text: "perro",
          pivot_text: "dog",
          notes: "el perro (m)",
        },
      },
    ]);
  });

  it("trims surrounding whitespace and skips blank lines", () => {
    const text = " es , perro , dog , \n\n  \nes,gato,cat,";
    const result = parseVocab(text, ",");

    expect(result.errors).toEqual([]);
    expect(result.rows.map((r) => r.line)).toEqual([1, 4]);
    expect(result.rows[0]?.term.foreign_text).toBe("perro");
  });

  it("flags a line missing a column by its line number", () => {
    const text = [
      "es,perro,dog,el perro (m)",
      "es,gato,cat", // notes column removed
      "es,pato,duck,",
    ].join("\n");
    const result = parseVocab(text, ",");

    expect(result.rows.map((r) => r.line)).toEqual([1, 3]);
    expect(result.errors).toEqual([
      { line: 2, reason: "expected at least 4 columns, found 3" },
    ]);
  });

  it("keeps a delimiter that appears inside the notes field", () => {
    const result = parseVocab("es,casa,house,la casa (f), a home", ",");

    expect(result.errors).toEqual([]);
    expect(result.rows[0]?.term.notes).toBe("la casa (f), a home");
  });

  it("flags a line whose identity fields are empty", () => {
    const result = parseVocab("es,,dog,", ",");

    expect(result.rows).toEqual([]);
    expect(result.errors).toEqual([
      {
        line: 1,
        reason: "foreign_lang, foreign_text and pivot_text must not be empty",
      },
    ]);
  });
});

const row = {
  line: 1,
  term: {
    foreign_lang: "es",
    foreign_text: "perro",
    pivot_text: "dog",
    notes: undefined,
  },
};

describe("canImport", () => {
  it("is true only with at least one row and no parse errors", () => {
    expect(canImport({ rows: [row], errors: [] })).toBe(true);
    expect(canImport({ rows: [], errors: [] })).toBe(false);
    expect(
      canImport({ rows: [row], errors: [{ line: 2, reason: "bad" }] }),
    ).toBe(false);
  });
});

describe("termsToImport", () => {
  it("pulls the Terms out of the parsed rows in order", () => {
    expect(termsToImport({ rows: [row], errors: [] })).toEqual([row.term]);
  });
});
