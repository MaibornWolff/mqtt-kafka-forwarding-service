use axum::{
    extract::Extension,
    http::{header::CONTENT_TYPE, HeaderMap, HeaderValue},
    routing::get,
    Router,
};
use prometheus_client::{encoding::text::encode, registry::Registry};
use std::sync::{Arc, Mutex};

async fn root() -> &'static str {
    "mqtt-kafka-forwarding-service"
}

async fn health() -> &'static str {
    return "OK";
}

async fn metrics(
    Extension(metrics_registry): Extension<Arc<Mutex<Registry>>>,
) -> (HeaderMap, Vec<u8>) {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/openmetrics-text; version=1.0.0; charset=utf-8"),
    );
    let mut buffer = vec![];
    let registry = metrics_registry.lock().unwrap();
    encode(&mut buffer, &registry).unwrap();
    (headers, buffer)
}

pub async fn api(metrics_registry: Registry) {
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .layer(Extension(Arc::new(Mutex::new(metrics_registry))));

    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
