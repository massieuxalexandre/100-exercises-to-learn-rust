// This is our last exercise. Let's go down a more unstructured path!
// Try writing an **asynchronous REST API** to expose the functionality
// of the ticket management system we built throughout the course.
// It should expose endpoints to:
//  - Create a ticket
//  - Retrieve ticket details
//  - Patch a ticket
//
// Use Rust's package registry, crates.io, to find the dependencies you need
// (if any) to build this system.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ticket_fields::{TicketDescription, TicketTitle};
use tokio::sync::RwLock;

use crate::data::{Ticket, TicketDraft, TicketPatch};
use crate::store::{TicketId, TicketStore};

pub mod data;
pub mod store;

type SharedStore = Arc<RwLock<TicketStore>>;

pub fn app() -> Router {
    let store = Arc::new(RwLock::new(TicketStore::new()));
    Router::new()
        .route("/", get(|| async {
            "Ticket API is running. Use POST /tickets, GET /tickets/{id} or PATCH /tickets/{id}."
        }))
        .route("/tickets", post(create_ticket))
        .route("/tickets/{id}", get(get_ticket).patch(patch_ticket))
        .with_state(store)
}

async fn create_ticket(
    State(store): State<SharedStore>,
    Json(draft): Json<TicketDraft>,
) -> Result<(StatusCode, Json<Ticket>), StatusCode> {
    TicketTitle::try_from(draft.title.as_str()).map_err(|_| StatusCode::BAD_REQUEST)?;
    TicketDescription::try_from(draft.description.as_str()).map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut store = store.write().await;
    let id = store.add_ticket(draft);
    let ticket = store.get(id).unwrap().clone();
    Ok((StatusCode::CREATED, Json(ticket)))
}

async fn get_ticket(
    State(store): State<SharedStore>,
    Path(id): Path<u64>,
) -> Result<Json<Ticket>, StatusCode> {
    let store = store.read().await;
    let ticket = store.get(TicketId(id)).ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(ticket.clone()))
}

async fn patch_ticket(
    State(store): State<SharedStore>,
    Path(id): Path<u64>,
    Json(patch): Json<TicketPatch>,
) -> Result<Json<Ticket>, StatusCode> {
    if let Some(title) = &patch.title {
        TicketTitle::try_from(title.as_str()).map_err(|_| StatusCode::BAD_REQUEST)?;
    }
    if let Some(description) = &patch.description {
        TicketDescription::try_from(description.as_str()).map_err(|_| StatusCode::BAD_REQUEST)?;
    }

    let mut store = store.write().await;
    let ticket = store.get_mut(TicketId(id)).ok_or(StatusCode::NOT_FOUND)?;
    if let Some(title) = patch.title {
        ticket.title = title;
    }
    if let Some(description) = patch.description {
        ticket.description = description;
    }
    if let Some(status) = patch.status {
        ticket.status = status;
    }
    Ok(Json(ticket.clone()))
}
