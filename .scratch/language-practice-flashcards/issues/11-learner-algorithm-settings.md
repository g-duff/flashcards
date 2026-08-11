# 11 — Learner Algorithm Settings

**What to build:** A Learner can view, edit, and reset their own spaced-repetition tuning parameters, independent of every other Learner, with new profiles starting from application defaults that don't retroactively override anyone's saved choices.

**Blocked by:** 10 — Spaced-repetition priority ranking.

**Status:** ready-for-agent

- [ ] Each new Learner's complete Learner Algorithm Settings are initialized from the YAML application defaults at profile creation.
- [ ] `GET /api/me/algorithm-settings` reads the current Learner's persisted settings.
- [ ] `PATCH /api/me/algorithm-settings` validates and saves updated values (rejecting invalid values with `400`); valid settings are accepted even if they would currently yield zero Eligible Entries.
- [ ] `POST /api/me/algorithm-settings/reset` copies the current YAML defaults into the Learner's record.
- [ ] Later changes to YAML defaults affect only new Learners; existing Learners keep their stored settings until they explicitly edit or reset.
- [ ] The priority formula from ticket 10 reads its coefficients from the acting Learner's persisted settings rather than hardcoded defaults.
- [ ] Client "Learner Settings" screen shows current settings, validates and saves edits, and offers reset-to-default.
- [ ] HTTP integration tests cover: default initialization on Learner creation, read, valid update, invalid-value rejection, reset-to-default, stored settings surviving a YAML default change, per-Learner isolation (one Learner's settings never affect another's).
