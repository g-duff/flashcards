# Domain Docs

Engineering skills should consume this repo's domain documentation as follows.

## Before exploring, read these

- `CONTEXT.md` at the repo root, if it exists.
- `docs/adr/` ADRs that touch the area being explored, if they exist.

If these files do not exist, proceed silently. The `/domain-modeling` skill creates them lazily when terms or decisions are resolved.

## Use the glossary's vocabulary

When naming a domain concept in an issue title, refactor proposal, or test, use the terminology defined in `CONTEXT.md`. If a needed concept is missing, note it for `/domain-modeling`.

## Flag ADR conflicts

If output contradicts an existing ADR, surface the conflict explicitly rather than silently overriding it.
