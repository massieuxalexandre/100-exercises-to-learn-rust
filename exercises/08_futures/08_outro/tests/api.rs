use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn request(app: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 10_000).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

#[tokio::test]
async fn create_get_and_patch() {
    let app = outro_08::app();
    let (status, created) = request(
        &app,
        "POST",
        "/tickets",
        json!({"title": "Bug", "description": "Fix it"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        created,
        json!({"id": 0, "title": "Bug", "description": "Fix it", "status": "ToDo"})
    );

    let (status, ticket) = request(&app, "GET", "/tickets/0", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ticket, created);

    let (status, updated) =
        request(&app, "PATCH", "/tickets/0", json!({"status": "InProgress"})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["status"], "InProgress");
    assert_eq!(updated["title"], created["title"]);
    assert_eq!(updated["description"], created["description"]);

    let (status, updated) = request(
        &app,
        "PATCH",
        "/tickets/0",
        json!({
            "title": "Fixed", "description": "All done", "status": "Done"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        updated,
        json!({"id": 0, "title": "Fixed", "description": "All done", "status": "Done"})
    );
    let (_, stored) = request(&app, "GET", "/tickets/0", Value::Null).await;
    assert_eq!(stored, updated);
    let (_, unchanged) = request(&app, "PATCH", "/tickets/0", json!({})).await;
    assert_eq!(unchanged, updated);
}

#[tokio::test]
async fn missing_ticket() {
    let app = outro_08::app();
    for method in ["GET", "PATCH"] {
        let (status, _) = request(&app, method, "/tickets/42", json!({})).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn invalid_fields_do_not_change_the_store() {
    let app = outro_08::app();
    for draft in [
        json!({"title": "", "description": "Valid"}),
        json!({"title": "a".repeat(51), "description": "Valid"}),
        json!({"title": "Valid", "description": ""}),
        json!({"title": "Valid", "description": "a".repeat(501)}),
    ] {
        let (status, _) = request(&app, "POST", "/tickets", draft).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let (_, original) = request(
        &app,
        "POST",
        "/tickets",
        json!({"title": "Bug", "description": "Fix it"}),
    )
    .await;
    assert_eq!(original["id"], 0);

    for patch in [
        json!({"title": "", "status": "Done"}),
        json!({"title": "Changed", "description": ""}),
        json!({"title": "a".repeat(51)}),
        json!({"description": "a".repeat(501)}),
    ] {
        let (status, _) = request(&app, "PATCH", "/tickets/0", patch).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let (_, stored) = request(&app, "GET", "/tickets/0", Value::Null).await;
        assert_eq!(stored, original);
    }
}

#[tokio::test]
async fn different_tickets_have_different_ids() {
    let app = outro_08::app();
    let (_, first) = request(
        &app,
        "POST",
        "/tickets",
        json!({"title": "First", "description": "First ticket"}),
    )
    .await;
    let (_, second) = request(
        &app,
        "POST",
        "/tickets",
        json!({"title": "Second", "description": "Second ticket"}),
    )
    .await;
    assert_ne!(first["id"], second["id"]);
    let (_, stored) = request(&app, "GET", "/tickets/0", Value::Null).await;
    assert_eq!(stored, first);
}

#[tokio::test]
async fn invalid_json_is_rejected() {
    let app = outro_08::app();
    let (status, _) = request(
        &app,
        "POST",
        "/tickets",
        json!({"title": "Missing description"}),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = request(&app, "PATCH", "/tickets/0", json!({"status": "Unknown"})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tickets")
                .header("content-type", "application/json")
                .body(Body::from("{"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
