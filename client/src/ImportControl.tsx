import { useState } from "react";
import type { ChangeEventHandler } from "react";
import { importTerms } from "./api/terms";
import type { ImportReport } from "./api/terms";
import { describeError } from "./errors";
import { canImport, parseVocab, termsToImport } from "./import";
import type { Optional, Result } from "./types/effects";
import { err, ok } from "./types/effects";

// Default delimiter for a `.csv`; the learner can change it in the field
// (e.g. `;`) and the preview re-parses as they type.
const DEFAULT_DELIMITER = ",";

type Outcome =
  | { status: "done"; report: ImportReport }
  | { status: "error"; message: string };

type ImportControlProps = {
  /** Called after a successful import so the Vocab screen can refetch the
   *  Term table. */
  onImported: () => void;
};

/** Vocab-screen bulk import: pick a delimited file, set the delimiter,
 *  see a parsed-row count and any unreadable lines by number, then send
 *  the parsed Terms as JSON. All file/delimiter handling is here. */
export const ImportControl = ({ onImported }: ImportControlProps) => {
  const [fileText, setFileText] = useState<Optional<string>>(undefined);
  const [fileName, setFileName] = useState<Optional<string>>(undefined);
  const [delimiter, setDelimiter] = useState(DEFAULT_DELIMITER);
  const [submitting, setSubmitting] = useState(false);
  const [outcome, setOutcome] = useState<Optional<Outcome>>(undefined);

  // Derived during render — changing the delimiter re-parses with no effect.
  const parsed =
    fileText === undefined ? undefined : parseVocab(fileText, delimiter);
  const ready = parsed !== undefined && canImport(parsed) && !submitting;

  const handleFile: ChangeEventHandler<HTMLInputElement> = (event) => {
    const file = event.target.files?.[0];
    if (file === undefined) return;
    setOutcome(undefined);
    void readText(file).then((result) => {
      if (result.ok) {
        setFileText(result.value);
        setFileName(file.name);
      } else {
        setFileText(undefined);
        setOutcome({ status: "error", message: result.error });
      }
    });
  };

  const handleImport = () => {
    if (!ready || parsed === undefined) return;
    setSubmitting(true);
    setOutcome(undefined);
    void importTerms(termsToImport(parsed)).then((result) => {
      setSubmitting(false);
      if (result.ok) {
        setOutcome({ status: "done", report: result.value });
        onImported();
      } else {
        setOutcome({ status: "error", message: describeError(result.error) });
      }
    });
  };

  return (
    <section className="import">
      <h2>Import</h2>
      <div className="import-controls">
        <input
          type="file"
          aria-label="vocab file"
          accept=".csv,.txt,text/csv,text/plain"
          onChange={handleFile}
        />
        <label>
          Delimiter
          <input
            aria-label="delimiter"
            className="delimiter"
            value={delimiter}
            onChange={(event) => setDelimiter(event.target.value)}
          />
        </label>
        <button type="button" onClick={handleImport} disabled={!ready}>
          {submitting ? "Importing…" : "Import"}
        </button>
      </div>

      {parsed !== undefined && (
        <div className="import-preview">
          <p className="muted">
            {fileName !== undefined && <>{fileName}: </>}
            {plural(parsed.rows.length, "row")} parsed
            {parsed.errors.length > 0 && (
              <> · {plural(parsed.errors.length, "line")} skipped</>
            )}
          </p>
          {parsed.errors.length > 0 && (
            <ul className="import-errors">
              {parsed.errors.map((error) => (
                <li key={error.line} className="error">
                  Line {error.line}: {error.reason}
                </li>
              ))}
            </ul>
          )}
        </div>
      )}

      {outcome?.status === "done" && (
        <p className="import-result">
          Imported {outcome.report.imported}, skipped {outcome.report.skipped}.
        </p>
      )}
      {outcome?.status === "error" && (
        <p className="error">{outcome.message}</p>
      )}
    </section>
  );
};

const plural = (n: number, word: string): string =>
  `${n} ${word}${n === 1 ? "" : "s"}`;

/** Read a picked file's text. A local outside-world effect, not an `api/`
 *  call, so it stays here — but it still hands back a `Result` rather than
 *  a rejecting promise, per the "Result everywhere" convention. */
const readText = (file: File): Promise<Result<string, string>> =>
  new Promise((resolve) => {
    const reader = new FileReader();
    reader.onload = () => resolve(ok(String(reader.result ?? "")));
    reader.onerror = () => resolve(err("Could not read that file."));
    reader.readAsText(file);
  });
