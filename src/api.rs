use axum::{
    http::{header::CONTENT_TYPE, HeaderMap, HeaderValue},
    routing::get,
    Router,
};

async fn root() -> &'static str {
    "mqtt-kafka-forwarding-service"
}

async fn health() -> &'static str {
    "OK"
}

async fn metrics() -> (HeaderMap, String) {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/openmetrics-text; version=1.0.0; charset=utf-8"),
    );
    let metrics = crate::metrics::metrics().await;
    (headers, metrics)
}

pub async fn api() {
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/metrics", get(metrics));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("Could not listen on port 8080");
    axum::serve(listener, app)
        .await
        .expect("Failed to start http api");
}
