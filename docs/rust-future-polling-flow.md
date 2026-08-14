# How Tokio Polls a Future

This guide follows one async operation from the server:

```rust
pub async fn list_learners(
    State(state): State<AppState>,
) -> Result<SuccessEnvelope<Vec<Learner>>, ErrorResponse> {
    let learners = repository::list(state.db()).await.map_err(internal_error)?;
    Ok(envelope::success(learners, Utc::now()))
}
```

The key idea is that `.await` does not block the Tokio worker thread. It
allows the current future to say, "I cannot continue yet; run something else
and wake me when I can continue."

## The complete flow

```text
┌─────────────────────────────────────────────────────────────────────┐
│ Tokio runtime has a ready `list_learners` task                      │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               v
┌─────────────────────────────────────────────────────────────────────┐
│ Tokio calls Future::poll(...)                                      │
│                                                                     │
│ The handler runs until it reaches:                                 │
│                                                                     │
│   repository::list(state.db()).await                               │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               v
┌─────────────────────────────────────────────────────────────────────┐
│ SQLx starts or checks the database operation                        │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                 ┌─────────────┴─────────────┐
                 │                           │
                 v                           v
┌──────────────────────────────┐  ┌───────────────────────────────────┐
│ Database result is ready      │  │ Database result is not ready      │
└──────────────┬───────────────┘  └──────────────────┬────────────────┘
               │                                     │
               v                                     v
┌──────────────────────────────┐  ┌───────────────────────────────────┐
│ Future returns                │  │ Future registers a Waker and      │
│ Poll::Ready(result)           │  │ returns Poll::Pending             │
└──────────────┬───────────────┘  └──────────────────┬────────────────┘
               │                                     │
               v                                     v
┌──────────────────────────────┐  ┌───────────────────────────────────┐
│ `.await` produces `learners` │  │ Tokio runs another ready task      │
│ and the handler continues    │  │ instead of blocking this thread   │
└──────────────┬───────────────┘  └──────────────────┬────────────────┘
               │                                     │
               v                                     v
┌──────────────────────────────┐  ┌───────────────────────────────────┐
│ Handler builds the envelope  │  │ Database becomes ready             │
│ and returns the HTTP result  │  └──────────────────┬────────────────┘
└──────────────┬───────────────┘                     │
               │                                     v
               v                    ┌───────────────────────────────────┐
┌──────────────────────────────┐     │ Waker tells Tokio this future     │
│ Future returns                │     │ should be polled again            │
│ Poll::Ready(Ok(response))    │     └──────────────────┬────────────────┘
└──────────────────────────────┘                        │
                                                        │
                                                        └───────┐
                                                                │
                                                                v
                                      ┌───────────────────────────────────┐
                                      │ Tokio calls Future::poll(...)      │
                                      │ again; execution resumes after    │
                                      │ `repository::list(...).await`     │
                                      └───────────────────────────────────┘
```

The two branches eventually meet: either the database was already ready, or
the future was polled again after the database woke it.

## What `poll` means

The standard library defines the `Future` protocol approximately like this:

```rust
trait Future {
    type Output;

    fn poll(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Self::Output>;
}
```

The server does not implement this trait by hand. The Rust compiler generates
an internal future state machine for each `async fn`, and Tokio calls `poll`
for it.

`Future`, `Poll`, `Context`, and `Waker` are Rust standard-library concepts.
Tokio supplies the runtime that repeatedly calls `poll` and schedules the
tasks.

## The two possible answers from `poll`

### `Poll::Ready`

```text
poll()
  |
  +--> Poll::Ready(value)
          |
          +--> this future is complete
          +--> `.await` evaluates to `value`
          +--> the async function continues
```

For the server example, `value` eventually contains the result of
`repository::list`: either a vector of learners or a repository error.

### `Poll::Pending`

```text
poll()
  |
  +--> Poll::Pending
          |
          +--> this future cannot make progress now
          +--> it stores the Waker from Context
          +--> control returns to Tokio
          +--> this task is not repeatedly polled in a tight loop
```

Returning `Pending` is a promise: something will call the stored waker when
the future may make progress. For database and network operations, the
runtime/library integration connects that wake-up to I/O readiness.

## The generated state machine

The compiler transforms the source-level sequence into something conceptually
similar to states:

```text
State 1: start list_learners
    |
    v
State 2: waiting for repository::list
    |
    +--> Pending: remember state 2 and return to Tokio
    |
    +--> Ready(learners): store learners
                          and move to state 3
    v
State 3: build envelope
    |
    v
State 4: return HTTP response
```

The future stores the local data it needs across the pause, including
references or owned values that remain valid after the function yields. This
is why Rust checks ownership and lifetimes around async blocks carefully.

## What happens on the worker thread?

```text
Tokio worker thread
  |
  +--> poll request A
  |       |
  |       +--> A returns Pending while SQLx waits
  |
  +--> poll request B
  |       |
  |       +--> B returns Ready
  |
  +--> poll request C
  |
  +--> later, poll request A after its Waker fires
```

This is cooperative scheduling. A task gives Tokio a chance to run other
tasks when it returns `Pending` at an incomplete `.await`.

An async function that performs a long CPU loop without reaching an incomplete
`.await` can keep a worker occupied. Async code is therefore not a guarantee
that every operation is automatically interruptible.

## Mapping the diagram to the server

| Diagram step | Server code or library |
| --- | --- |
| Runtime starts | Tokio's `#[tokio::main]` in `server/src/main.rs` |
| Request future is created | Axum's `axum::serve(listener, app)` |
| Handler reaches an await | `server/src/routes/learners.rs` |
| Database future | SQLx's `fetch_all(pool)` in `server/src/learners/repository.rs` |
| Polling and scheduling | Tokio runtime |
| Wake-up after I/O | SQLx/Tokio integration |
| Final HTTP response | Axum converts the handler result into a response |

The source code looks like ordinary sequential Rust because the compiler and
runtime handle the state machine and polling mechanics underneath.

## A practical reading rule

Whenever you see:

```rust
some_async_operation().await
```

read it as:

```text
1. Ask the operation whether it can finish.
2. If yes, continue with its value.
3. If no, save the current state and return Pending.
4. Let the runtime run other work.
5. Resume after the operation wakes this future.
```

