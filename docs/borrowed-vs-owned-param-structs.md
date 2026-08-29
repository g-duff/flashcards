# Borrowed vs owned fields in parameter structs (Rust)

A note on why `NewSession<'a>` in `server/src/practice_sessions/repository.rs`
carries an explicit lifetime, what that lifetime does, and when the pattern is
worth it.

## The code in question

```rust
pub struct NewSession<'a> {
    pub learner_id: LearnerId,
    pub category_id: CategoryId,
    pub direction: TranslationDirection,
    pub requested_question_count: u32,
    pub questions: &'a [GeneratedQuestion],
}

pub async fn create(
    pool: &SqlitePool,
    new: NewSession<'_>,
    now: DateTime<Utc>,
) -> Result<PracticeSession, RepositoryError> { ... }
```

## What the lifetime is doing

`NewSession` stores a borrowed field (`questions: &'a [GeneratedQuestion]`). Any
struct that holds a reference **must** name that reference's lifetime as a
generic parameter — Rust has no lifetime elision for struct definitions. So
`<'a>` is not decoration and not an optimisation knob; it is the mandatory
syntax for "this struct borrows a `[GeneratedQuestion]` slice it does not own."

`'a` is the compiler's name for *the region where that borrow is live*. It ties
the two lifetimes together and enforces:

```
   lifetime of NewSession   ⊆   'a   ⊆   lifetime of `questions`
   ───────────────────────       ──       ──────────────────────
   the borrower                          the owner

   "the borrower must not outlive the owner"
```

The `create` signature uses the anonymous form `NewSession<'_>`: "there is a
lifetime here, infer it, it does not need to relate to the return type." The
returned `PracticeSession` is fully owned, so nothing borrowed leaks out.

This is the most benign shape of explicit lifetime there is: one borrowed field,
one lifetime parameter, no bounds, no `'a: 'b` outlives relations, no lifetime
in the return type. Wariness about explicit lifetimes is better aimed at those
more complex forms.

## 1. What `NewSession<'a>` actually sets up

```
   http handler scope
   ┌─────────────────────────────────────────────────────────────┐
   │                                                             │
   │  let questions: Vec<GeneratedQuestion> = generate_...();     │
   │      │                                                       │
   │      │  owns the heap buffer                                 │
   │      ▼                                                       │
   │  ┌───────────────┐        heap                               │
   │  │ ptr ──────────┼──────▶ [ Q0 | Q1 | Q2 | ... ]             │
   │  │ len, cap      │                                           │
   │  └───────────────┘                                           │
   │      ▲                                                       │
   │      │  &questions   (a shared borrow, no ownership)         │
   │      │                                                       │
   │  NewSession {                                                │
   │      questions: &'a [GeneratedQuestion]  ─┐                  │
   │      ...                                  │                  │
   │  }                                        │  'a = this span  │
   │      │                                    │                  │
   │      ▼                                    │                  │
   │  repository::create(db, new_session, now).await              │
   │      │  consumes NewSession here          │                  │
   │      ▼                                   ─┘                  │
   │  // NewSession gone. questions still valid, still owned.      │
   │  tracing::info!(... session ...);                            │
   │                                                             │
   └─────────────────────────────────────────────────────────────┘
        questions dropped here (end of scope), heap buffer freed
```

The struct declaration `struct NewSession<'a>` is just Rust forcing you to
*name* that relationship, because a struct with a `&` field cannot exist without
one.

## 2. Option 1 — own the Vec, no lifetime

```
   let questions: Vec<GeneratedQuestion> = generate_...();
   ┌───────────────┐  heap
   │ ptr ──────────┼─▶ [ Q0 | Q1 | Q2 ]
   └───────────────┘
       │
       │  MOVE (ptr/len/cap copied, original variable dead)
       ▼
   NewSession {
       questions: Vec<GeneratedQuestion>   ← owns the buffer now
   }
       │
       │  MOVE again into create()
       ▼
   repository::create(db, new_session, now)
       └─ create owns the buffer, drops it when done

   struct NewSession { ... }   ← no <'a>, nothing to relate, nothing to outlive
```

No borrow => no lifetime => no `'a` anywhere. Cost: one `Vec` move (three
machine words), which the caller here does not even notice because it never
touches `questions` again after building `NewSession`.

## 3. Why the vocab-entry structs keep the borrow

`NewEntry<'a>` and `NewBulkEntry<'a>` in
`server/src/vocabulary_entries/repository.rs` use the same borrowed-field
pattern, and there it pays off:

```
   BORROW pays off when the owner outlives the call AND is reused:

   let payload: CreateEntryRequest = parse_json(body);   // owns the Strings
        │
        ├──&payload.source_text──▶ NewEntry { source_text: &'a str, ... }
        │                              │
        │                              ▼
        │                          repository::insert(db, new_entry)
        │
        └──────────────▶ still needed: log it, return it in the response,
                          build the normalized identity, etc.

   If NewEntry took String, the caller would have to .clone() every field
   to keep its own copy. The borrow avoids that.
```

```
   NewSession is the opposite shape:

   let questions = generate(...);   ─┐  built solely to hand to create()
       └──▶ NewSession { &questions }│  never read again afterwards
            └──▶ create()           ─┘

   Nothing to protect from a move => the borrow buys nothing here.
```

## Takeaway

- Borrow in a parameter struct when the caller still owns and *reuses* the data
  after the call (the vocab payload case).
- Own it when the struct is a one-way bag of values you build and immediately
  pass down (the `NewSession` case).
- `NewSession<'a>` picked "borrow" by copying the neighbouring `NewEntry<'a>` /
  `NewBulkEntry<'a>` / `GenerateSessionInput<'a>` pattern, not because its own
  usage calls for it. If the lone lifetime parameter bothers you, `NewSession`
  is the safe one to switch to an owned `Vec<GeneratedQuestion>`: the caller at
  `server/src/http/practice_sessions.rs` drops `questions` right after the call
  and will not notice.
