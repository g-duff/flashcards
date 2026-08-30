# 05: Bulk import

**What to build:** The way vocabulary gets into the app in bulk — for first-run setup and for adding batches later. The learner picks a delimited text file and sets the delimiter (default `,`); the browser parses it into a list of Terms, shows how many rows it found and flags any malformed lines by number; the learner confirms and the Terms are sent to the server as JSON. Re-importing a file that overlaps an earlier one silently skips the Terms already present. The result reports how many were imported and how many skipped.

The server only ever receives JSON — all file and delimiter handling is in the browser.

**Blocked by:** 03 (the Term→two-Cards insert path the import shares).

**Status:** ready-for-agent

## Acceptance criteria

- [ ] `POST /terms/import` accepts a JSON array of `NewTerm`. Each element is validated (same rules as `POST /terms`); one bad element → `400 {error}` naming the element index, nothing imported (all-or-nothing parse/validate).
- [ ] Valid elements are inserted via ticket 03's shared path, so each new Term also gets its two Cards. Dedup is `INSERT ... ON CONFLICT(id) DO NOTHING`; response is `{imported: N, skipped: M}` where `skipped` counts the conflicts.
- [ ] `openapi.yaml` updated with `/terms/import` and its request/response schemas.
- [ ] Import control on the Vocab screen: file picker + a delimiter text field defaulting to `,`. On file select, the browser parses to `[NewTerm]` (columns per `dev/sample-vocab.csv`), shows a parsed-row count, and lists any unparseable lines with their line numbers. A disabled "Import" button until there is at least one valid row and no parse errors.
- [ ] After import: show `imported` / `skipped` from the response and refresh the Term table.
- [ ] `dev/sample-vocab.csv` added — ~15 Spanish Terms (`foreign_lang,foreign_text,pivot_text,notes` with the default `,` delimiter), usable as-is for first-run setup and as a test fixture.
- [ ] Backend tests: happy path, dedup on re-import, `400` on a bad element with nothing persisted. Frontend tests: parse with the default delimiter, parse with a custom delimiter, a malformed line reported by number, the disabled-button rule.

## How to test it yourself

1. `./dev/up.sh` with an empty deck. Vocab screen shows no Terms.
2. Use the Import control, pick `dev/sample-vocab.csv`, leave the delimiter as `,`. Preview shows ~15 rows, no errors. Import → result says `imported: 15, skipped: 0`; the table fills.
3. Import the same file again → `imported: 0, skipped: 15`; no duplicate rows.
4. Enter Practice — there are ~30 due Cards (two per imported Term). The imported Terms behave exactly like hand-added ones.
5. Make a copy of the file with one row missing a column; import it → the preview flags that line by number and the Import button stays disabled.
6. Make a copy using `;` as the delimiter, set the delimiter field to `;`, import → parses correctly.
7. `curl -X POST .../api/terms/import -d '[{"foreign_lang":"es","foreign_text":"","pivot_text":"x"}]'` → `400` naming index 0; `curl .../api/terms` unchanged.
8. `./dev/down.sh` + `./dev/up.sh` — the imported deck is still there.
