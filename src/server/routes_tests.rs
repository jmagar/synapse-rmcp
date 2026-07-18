use axum::{body::Body, http::Request};
use tower::ServiceExt;

use super::router;

#[tokio::test]
async fn openapi_json_is_served_without_auth() {
    let response = router(crate::testing::loopback_state())
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let content_type = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .expect("content-type should be set");
    assert!(content_type.starts_with("application/json"));
}

#[tokio::test]
async fn concurrency_limit_sheds_overload_without_queueing() {
    use std::{convert::Infallible, sync::Arc};
    use tower::{Layer, service_fn};

    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let service = service_fn({
        let entered = Arc::clone(&entered);
        let release = Arc::clone(&release);
        move |_request: Request<Body>| {
            let entered = Arc::clone(&entered);
            let release = Arc::clone(&release);
            async move {
                entered.notify_one();
                release.notified().await;
                Ok::<_, Infallible>(axum::response::Response::new(Body::empty()))
            }
        }
    });
    let limited = super::ConcurrencyLimitLayer::new(1).layer(service);
    let first = tokio::spawn(limited.clone().oneshot(Request::new(Body::empty())));
    entered.notified().await;

    let overloaded = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        limited.oneshot(Request::new(Body::empty())),
    )
    .await
    .expect("overload response must be prompt")
    .unwrap();
    assert_eq!(
        overloaded.status(),
        axum::http::StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        overloaded
            .headers()
            .get(axum::http::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok()),
        Some("1")
    );

    release.notify_one();
    assert!(first.await.unwrap().is_ok());
}
