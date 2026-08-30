# Flashcards

A single-user language-learning flashcards app: capture vocabulary as
translation pairs, then practise each pair in both directions on a
spaced-repetition schedule. Built to start Spanish practice quickly and
grow from there.

## Language

### Vocabulary

**Term**:
One vocabulary pair — a `foreign_text` in some foreign language and its
`pivot_text` in the pivot language. A Term's two texts are its identity
and never change; only its `notes` can be edited.
_Avoid_: word, entry, vocab item, translation, translation pair, card

**Foreign language**:
The language being learned. Recorded per Term (`foreign_lang`), so the
deck can hold more than one over time.
_Avoid_: target language, L2, second language

**Pivot language**:
The one language the learner already knows and every Term shares — the
side a Term is translated *to*. A single app-wide setting, not stored per
Term.
_Avoid_: native language, L1, base language, known language

**Notes**:
Free-text annotation on a Term (article for a noun, an irregular
conjugation, a mnemonic). The only mutable part of a Term.
_Avoid_: hint, comment, description

### Practice

**Card**:
One direction of a Term. Every Term has exactly two: one prompting with
the foreign text, one prompting with the pivot text. A Card carries its
own schedule, independent of its sibling.
_Avoid_: side, face, question

**Prompt side**:
Which of a Term's two texts a Card shows. `foreign` means the learner
sees the foreign text and recalls the pivot text; `pivot` means the
reverse.
_Avoid_: direction, orientation, mode

**Recognition**:
The Card whose prompt side is `foreign` — see the foreign word, recall
its meaning. The easier direction.
_Avoid_: passive recall, comprehension

**Production**:
The Card whose prompt side is `pivot` — see the known word, produce the
foreign word. The harder direction.
_Avoid_: active recall, active production

**Scheduler**:
The strategy that decides when a Card is next due and how a Review
changes that. Pluggable; the current one is Leitner boxes.
_Avoid_: algorithm, spaced-repetition engine, SRS

**Leitner boxes**:
The current Scheduler. Each Card sits in a numbered box; a passing Review
promotes it one box and pushes its due date further out, a failing Review
sends it back to the first box.
_Avoid_: SM-2, FSRS, intervals

**Due**:
A Card is due when its scheduled date has arrived or passed. A practice
run draws from the due Cards.
_Avoid_: pending, ready, scheduled

**Review**:
One graded attempt at a Card during practice. Recorded permanently and
never changed — the history a future Scheduler could be rebuilt from.
_Avoid_: attempt, answer, result, grade

**Rating**:
The learner's self-assessment of a Review: `pass` or `fail`. The
vocabulary leaves room for a finer scale later.
_Avoid_: score, grade, mark

**Practice run**:
One sitting in which the learner works through the due Cards. It is not
recorded — only the Reviews it produces are.
_Avoid_: session, round, quiz

## How the nouns relate

```
      TERM  ──<  CARD  ──<  REVIEW

  a vocab pair   one direction     one graded
  of foreign +   of a term, with   attempt at a
  pivot text;    its own           card during a
  text is        schedule          practice run
  immutable
                 exactly 2 per     0..n per card,
                 term:             kept forever
                 recognition
                 + production
```

- A **Term** has exactly two **Cards** — recognition and production.
  Deleting a Term deletes both.
- A **Card** accumulates **Reviews**, one per graded attempt. Deleting a
  Card (only ever via its Term) deletes its Reviews.
- Nothing references a **practice run**; it exists only while the learner
  is practising.
