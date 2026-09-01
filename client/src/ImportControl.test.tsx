import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ok } from "./types/effects";

vi.mock("./api/terms", () => ({
  importTerms: vi.fn(),
}));

const api = await import("./api/terms");
const { ImportControl } = await import("./ImportControl");

const file = (body: string, name = "vocab.csv") =>
  new File([body], name, { type: "text/csv" });

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("ImportControl", () => {
  it("keeps Import disabled until a file parses to at least one row with no errors", async () => {
    const user = userEvent.setup();
    render(<ImportControl onImported={vi.fn()} />);

    const button = screen.getByRole("button", { name: "Import" });
    expect(button).toBeDisabled();

    // A file with one good row and one malformed row: still disabled.
    await user.upload(
      screen.getByLabelText("vocab file"),
      file("es,perro,dog,\nes,gato,cat"),
    );
    expect(await screen.findByText(/1 row parsed/)).toBeInTheDocument();
    expect(
      screen.getByText("Line 2: expected at least 4 columns, found 3"),
    ).toBeInTheDocument();
    expect(button).toBeDisabled();
  });

  it("enables Import for a clean file and sends the parsed Terms as JSON", async () => {
    vi.mocked(api.importTerms).mockResolvedValue(
      ok({ imported: 2, skipped: 1 }),
    );
    const onImported = vi.fn();
    const user = userEvent.setup();
    render(<ImportControl onImported={onImported} />);

    await user.upload(
      screen.getByLabelText("vocab file"),
      file("es,perro,dog,el perro (m)\nes,gato,cat,"),
    );

    const button = screen.getByRole("button", { name: "Import" });
    expect(await screen.findByText(/2 rows parsed/)).toBeInTheDocument();
    expect(button).toBeEnabled();

    await user.click(button);

    expect(api.importTerms).toHaveBeenCalledWith([
      {
        foreign_lang: "es",
        foreign_text: "perro",
        pivot_text: "dog",
        notes: "el perro (m)",
      },
      {
        foreign_lang: "es",
        foreign_text: "gato",
        pivot_text: "cat",
        notes: undefined,
      },
    ]);
    expect(
      await screen.findByText("Imported 2, skipped 1."),
    ).toBeInTheDocument();
    expect(onImported).toHaveBeenCalledOnce();
  });

  it("re-parses with a custom delimiter", async () => {
    const user = userEvent.setup();
    render(<ImportControl onImported={vi.fn()} />);

    await user.upload(
      screen.getByLabelText("vocab file"),
      file("es;perro;dog;el perro (m)"),
    );
    // With the default comma the single line has the wrong column count.
    expect(await screen.findByText(/0 rows parsed/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Import" })).toBeDisabled();

    const delimiter = screen.getByLabelText("delimiter");
    await user.clear(delimiter);
    await user.type(delimiter, ";");

    expect(await screen.findByText(/1 row parsed/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Import" })).toBeEnabled();
  });
});
