# Pragmatic Functional Rust: Coding Standards & Guidelines

This document outlines the idiomatic coding standards for our Rust codebase. Our goal is to blend Rust's zero-cost abstractions, memory safety, and performance with a **pragmatic functional programming style**—maximizing immutability, type safety, and declarative data flows while avoiding dogmatic over-engineering.

---

## 1. Module Structure and File Organization

We follow the modern Rust module layout (introduced in Rust 2018) to keep directory structures clean, flat, and easy to navigate.

* **Avoid `mod.rs` Files:** Do not use `mod.rs` files for module entry points. They clutter file trees and make file navigation confusing when multiple files share the same name across directories.
* **Use `foo.rs` + `foo/` (Modern Style):** Pair a module file with a directory of the same name when sub-modules are required.
* If a module `users` grows sub-modules (like `repository` or `service`), place the root logic in `users.rs` and the sub-modules inside a `users/` directory.


```text
// Recommended File Structure
src/
├── main.rs
├── users.rs              <-- Public interface & module declaration for users
└── users/
    ├── repository.rs     <-- Sub-module: accessed via `users::repository`
    └── service.rs        <-- Sub-module: accessed via `users::service`

```


* **Module Declaration Syntax:** In `main.rs` (or your crate root), declare the module and its sub-modules cleanly:
```rust
// src/main.rs
mod users; // Automatically looks for src/users.rs

```


And inside `src/users.rs`, declare its internal children:
```rust
// src/users.rs
pub mod repository; // Automatically looks for src/users/repository.rs
pub mod service;    // Automatically looks for src/users/service.rs

```
---

## 2. Immutability and Expression-Oriented Design

Rust is expression-oriented by design. We lean heavily into this to minimize mutable state and side effects.

* **Default to Immutability:** Always use `let` unless mutation is strictly necessary. If you must use `let mut`, keep the mutable scope as localized as possible.
* **Prefer Expressions over Statements:** Write blocks, `if` statements, and `match` arms so they evaluate to a value rather than assigning to a mutable variable.
```rust
// Good: Expression-oriented
let status = match user.role {
    Role::Admin => Status::Active,
    Role::Guest => Status::Pending,
};

// Avoid: Imperative mutation
let status;
if user.role == Role::Admin {
    status = Status::Active;
} else {
    status = Status::Pending;
}

```



---

## 3. Type-Driven Development and Algebraic Data Types (ADTs)

Make illegal states unrepresentable by leveraging Rust's powerful enum and struct systems.

* **Use Newtypes for Domain Primitives:** Avoid primitive obsession. Wrap primitives in distinct types to enforce correctness at compile time.
```rust
pub struct UserId(Uuid);
pub struct EmailAddress(String);

```


* **Leverage Enums for State Modeling:** Use enums with data payloads to represent mutually exclusive states, rather than optional fields that require runtime checks.
```rust
// Good
pub enum OrderState {
    Pending { placed_at: DateTime<Utc> },
    Shipped { tracking_number: String },
    Cancelled { reason: String },
}

```



---

## 4. Error Handling and Composition

Errors are values. We treat them explicitly using `Result` and `Option`, combining them with functional combinators and the `?` operator.

* **No Unchecked Panics:** Never use `.unwrap()` or `.expect()` in production code paths unless a panic is genuinely unrecoverable (e.g., static initialization failure).
* **Embrace Combinators:** Use functional combinators (`map`, `and_then`, `filter`, `ok_or_else`) to transform `Result` and `Option` chains fluently.
```rust
// Good
let display_name = current_user()
    .map(|u| u.profile)
    .and_then(|p| p.nickname)
    .unwrap_or_else(|| "Anonymous".to_string());

```


* **Unified Error Enums:** Use libraries like `thiserror` to define domain-specific error types that implement `std::error::Error`.

---

## 5. Collections and Declarative Iteration

Data transformation should be declarative, lazy, and readable.

* **Prefer Iterators Over Loops:** Use iterator pipelines (`.iter()`, `.map()`, `.filter()`, `.fold()`, `.collect()`) for data transformations instead of manual `for` loops with mutable accumulators.
```rust
// Good: Declarative transformation
let active_emails: Vec<Email> = users
    .into_iter()
    .filter(|u| u.is_active())
    .map(|u| u.email)
    .collect();

```


* **Keep Chains Readable:** If an iterator chain exceeds 3-4 operations or involves complex closures, extract the closure into a well-named helper function or split it into intermediate variables.

---

## 6. Functional Core, Imperative Shell

This project strictly adheres to the **Functional Core, Imperative Shell** architecture pattern. All code must be cleanly separated into pure business logic (the core) and side-effect-heavy orchestration (the shell).


* **The Functional Core:**
    * Contains all business logic, data transformations, domain models, and decision-making rules.
    * Must be **pure**: given the same inputs, it must always return the exact same outputs without any side effects.
    * Must **not** perform I/O, mutate global state, write to databases, make network requests, log directly, or read system clocks/environment variables.
    * Implemented primarily using pure functions, immutable data structures, and standard Rust enums/structs.

* **The Imperative Shell:**
    * Acts as the thin outer layer (e.g., CLI entry points, web server handlers, event listeners, database drivers).
    * Responsible for all **side effects**: reading/writing files, network calls, database queries, reading environment variables, and handling system time.
    * Fetches data, passes it into the functional core for processing, and then takes the output of the core to perform the necessary actions.

---

### 2. Rules for the AI Coding Agent

When generating or refactoring Rust code, you must obey these structural rules:

#### Rule 1: Isolate Side Effects
* Do not mix I/O operations with business calculations inside the same function. 
* If a function needs to fetch data and process it, split it into two parts:
    1. An imperative function that performs the I/O.
    2. A pure function (in the core) that accepts the raw data and returns the processed result.

#### Rule 2: Leverage Rust's Type System for Purity
* Core logic should express domain rules through types (e.g., custom structs, newtype patterns, and enums) rather than throwing runtime exceptions or performing unhandled side effects.
* Use `Result<T, E>` and `Option<T>` extensively in the core to handle expected failures and missing states explicitly.

#### Rule 3: Dependency Injection via Parameters
* If the core needs configuration or external state to make a decision, pass it in as a parameter or function argument rather than having the core fetch it dynamically.


### 3. Code Examples

### ❌ Incorrect: Mixed Logic and Side Effects
```rust
// BAD: Business logic is tightly coupled with I/O and mutations
pub fn process_user_signup(username: &str) -> Result<(), String> {
    if username.is_empty() {
        return Err("Username cannot be empty".to_string());
    }
    
    // Side effect inside core logic
    let connection = std::fs::read_to_string("db_config.txt").map_err(|e| e.to_string())?;
    println!("Connecting to database...");
    
    Ok(())
}

```

#### ✔️ Correct: Functional Core, Imperative Shell

```rust
// --- THE FUNCTIONAL CORE ---
pub mod core {
    #[derive(Debug, PartialEq)]
    pub enum SignupError {
        EmptyUsername,
    }

    // Pure validation and data transformation logic
    pub fn validate_signup(username: &str) -> Result<(), SignupError> {
        if username.trim().is_empty() {
            return Err(SignupError::EmptyUsername);
        }
        Ok(())
    }
}

// --- THE IMPERATIVE SHELL ---
pub mod shell {
    use super::core::validate_signup;

    // Handles I/O and orchestration
    pub fn handle_user_signup(username: &str) {
        match validate_signup(username) {
            Ok(()) => println!("Success: User is valid, proceeding with database save."),
            Err(e) => eprintln!("Error: {:?}", e),
        }
    }
}
```


While strict functional purity is impossible in a systems language, we strive to separate **pure business logic** from **effectful infrastructure**.

* **Isolate I/O and Mutation:** Keep core business logic inside pure functions (taking inputs and returning values/errors without touching databases, network, or clocks). Push I/O to the edges of the application (e.g., handlers, repositories).
* **Dependency Injection via Traits:** Pass behaviors (like repositories or external clients) into pure functions via traits rather than relying on global state or mutable shared references.

---


## Testing

Functional core: all code in the functional core must be unit-tested, with unit tests in the same file as the production code. The unit tests must prioritise the use-cases described in user-stories.

---

> **Summary Guideline:** Write code that compiles cleanly, expresses domain logic through types rather than documentation, and transforms data through predictable, immutable pipelines.

