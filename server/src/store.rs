//! In-memory card store. The imperative shell's state: a `Vec<Card>`
//! behind a mutex, plus a monotonic id counter.
//!
//! No persistence — a process restart resets the deck to the seed. When
//! this needs to survive a redeploy, replace it with a SQLite-backed
//! store (the Sandy Bank downloads app is the worked example) keeping the
//! same method surface.

use std::sync::{Arc, Mutex};

use crate::model::Card;

#[derive(Clone)]
pub struct Store {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    cards: Vec<Card>,
    next_id: u64,
}

impl Store {
    /// A fresh store carrying a few sample cards so the UI has something
    /// to show on first load.
    pub fn seeded() -> Self {
        let seed = [
            ("What does SPA stand for?", "Single-page application"),
            ("nginx: prefix vs regex location matching order?", "Prefix by longest match; regex in file order"),
            ("musl vs glibc for the pi build — why musl?", "Fully static binary, no runtime glibc dependency"),
        ];
        let cards = seed
            .iter()
            .enumerate()
            .map(|(i, (front, back))| Card {
                id: i as u64 + 1,
                front: (*front).to_string(),
                back: (*back).to_string(),
            })
            .collect::<Vec<_>>();
        let next_id = cards.len() as u64 + 1;
        Self {
            inner: Arc::new(Mutex::new(Inner { cards, next_id })),
        }
    }

    /// Every card, in insertion order.
    pub fn list(&self) -> Vec<Card> {
        self.inner.lock().expect("store mutex poisoned").cards.clone()
    }

    pub fn get(&self, id: u64) -> Option<Card> {
        self.inner
            .lock()
            .expect("store mutex poisoned")
            .cards
            .iter()
            .find(|c| c.id == id)
            .cloned()
    }

    /// Insert a card, assigning it the next id. Caller is responsible for
    /// having validated the sides (see [`crate::core::validate_new_card`]).
    pub fn add(&self, front: String, back: String) -> Card {
        let mut inner = self.inner.lock().expect("store mutex poisoned");
        let id = inner.next_id;
        inner.next_id += 1;
        let card = Card { id, front, back };
        inner.cards.push(card.clone());
        card
    }
}
