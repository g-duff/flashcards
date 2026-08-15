# 02 — Learner creation, selection & cookie identity

**What to build:** A Learner can create a durable local profile from the Home screen, select an existing one, and have the app remember them via a secure cookie across restarts — without any request being able to impersonate another Learner by supplying an arbitrary ID.

**Blocked by:** 01 — Project scaffolding.

**Status:** done

- [x] `POST /api/learners` creates a Learner with a unique display name and sets the current-learner cookie; `GET /api/learners` lists Learners for profile selection.
- [x] Learner names are compared case-insensitively after trimming whitespace for uniqueness, while display casing is preserved; duplicate names return a `409` conflict.
- [x] `POST /api/session/learner` selects an existing Learner and sets the cookie; `DELETE /api/session/learner` clears it.
- [x] The current-learner cookie stores the durable Learner ID, is host-only, `HttpOnly`, `SameSite=Lax`, and has an explicit long expiry.
- [x] An invalid or deleted-profile cookie is cleared by the server and the client redirects to Home.
- [x] Learner-scoped endpoints derive identity from the cookie; supplying a different Learner ID in a request body does not let a caller act as another Learner.
- [x] Home screen: create a Learner, select an existing Learner, display the current Learner.
- [x] HTTP integration tests cover: creation, duplicate-name conflict, selection, cookie attributes, invalid-cookie clear + redirect.
