# 04: Practice screen

**What to build:** The screen where the learner actually practises. It pulls the due Cards, shows one prompt at a time, reveals the answer on demand, takes a Pass / Fail self-grade, records it, and moves to the next Card. A Card marked Fail comes back later in the same sitting. When the queue is empty the learner sees a summary of how the sitting went. A "N due" indicator is visible from wherever the app starts so the learner knows there is something to do.

**Blocked by:** 03 (`GET /cards?due_before=`, `POST /cards/{id}/reviews`).

**Status:** ready-for-agent

## Acceptance criteria

- [ ] On entering Practice, the client fetches `GET /cards?due_before=<now>&limit=20`.
- [ ] Per Card: show `prompt`; a control reveals `answer` (and `notes` if present); then **Pass** and **Fail** buttons. Choosing one `POST`s `{rating}` to `/cards/{id}/reviews` and advances.
- [ ] A Card graded **Fail** is re-inserted later into the remaining in-memory queue for this sitting (not necessarily next).
- [ ] When the queue is exhausted: an end-of-run summary — counts of passed, failed, and total seen — plus a way back to the start.
- [ ] A persistent "N due" badge (count from a `due_before=<now>` query) is shown on the app's landing view; it decreases as Cards are passed.
- [ ] Empty state: entering Practice with nothing due shows a clear "nothing due right now" message, not a blank screen or an error.
- [ ] Network / HTTP / malformed-response failures are surfaced using the existing `ApiError` + `describeError` pattern, not swallowed.
- [ ] Client conventions from `client/CODING_STANDARDS.md` upheld (`Optional`/`Result`, discriminated-union view state, pure helpers). Frontend tests cover: reveal-then-grade flow, a failed Card reappearing, the summary counts, and the empty state.

## How to test it yourself

1. `./dev/up.sh`. Add three Terms via Vocab so there are six due Cards. The landing view shows "6 due".
2. Enter Practice. First prompt shows, answer hidden. Reveal it — answer (and notes if any) appear.
3. Press **Pass**. Next prompt appears; badge now "5 due".
4. Press **Fail** on the next one. Keep going — that failed Card shows up again before the sitting ends.
5. Finish the queue. Summary shows passed / failed / seen totals adding up correctly.
6. Go back and enter Practice again immediately — only the Cards still due (the ones you failed, plus any not yet promoted) are offered.
7. Pass everything. Enter Practice once more — "nothing due right now".
8. Stop the backend (`./dev/down.sh`) and open Practice — the error is shown via the normal error UI, not a crash.
