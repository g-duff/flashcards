//! Imperative shell for the HTTP layer: the axum router, the shared
//! [`AppState`], the request handlers ([`handlers`]), and the
//! error-to-status mapping ([`error`]). The pure validation logic the
//! handlers lean on lives in [`crate::core`].

pub mod error;
pub mod handlers;

use crate::store::Store;

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
}

/// Builds the application router. `main` owns constructing the state;
/// this owns the URL-to-handler map.
///
/// Routes are as the binary sees them — nginx strips the
/// `/flashcards/api/` prefix before proxying, and serves
/// `/flashcards/openapi.yaml` from `/openapi.yaml` here.
pub fn router(state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/healthz", axum::routing::get(handlers::healthz))
        .route("/openapi.yaml", axum::routing::get(handlers::openapi))
        .route(
            "/cards",
            axum::routing::get(handlers::list_cards).post(handlers::create_card),
        )
        .route("/cards/:id", axum::routing::get(handlers::get_card))
        .with_state(state)
}
