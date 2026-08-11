# 10 — Spaced-repetition priority ranking

**What to build:** Session generation stops treating all Eligible Entries equally: entries with repeated incorrect answers or stale review come back first, while material with a strong correct streak backs off — all computed from exact elapsed UTC durations.

**Blocked by:** 08 — Answer submission with transactional progress tracking & auto-completion; 07 — Practice Session generation & eligibility.

**Status:** ready-for-agent

- [ ] Priority for each Eligible Entry+direction is computed as: `base_priority`, plus `elapsed_since_last_correct_days * time_decay_factor` once `elapsed_since_last_correct > min_interval_before_retest`, plus `total_incorrect_count * incorrect_weight`, minus `deprioritize_duration_days * 10` when `current_correct_streak >= correct_streak_threshold`.
- [ ] The hard retest cooldown (from ticket 07's eligibility rule) and this priority formula are evaluated consistently against the same elapsed-UTC-duration logic, correct across daylight-saving boundaries.
- [ ] Session generation (ticket 07's entry selection) now selects the highest-priority Eligible Entries first, then shuffles question order within the chosen set.
- [ ] All duration math uses exact elapsed UTC durations, not local calendar dates.
- [ ] Focused deterministic unit tests cover priority-formula edge cases (exact min-interval boundary, streak-threshold boundary, zero/negative elapsed time) independent of full HTTP setup, plus HTTP-level tests confirming higher-priority entries are preferred in generated sessions.
