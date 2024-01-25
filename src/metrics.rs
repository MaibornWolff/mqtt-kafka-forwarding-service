use lazy_static::lazy_static;
use prometheus_client::encoding::text::encode;
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::Registry;
use tokio::sync::Mutex;

#[derive(Clone, Hash, PartialEq, Eq, Debug, EncodeLabelSet)]
pub struct MetricLabels {
    pub topic: String,
}

lazy_static! {
    static ref REGISTRY: Mutex<Registry> = Mutex::new(<Registry>::default());
    pub static ref COUNT_MQTT_RECEIVED: Family<MetricLabels, Counter> =
        Family::<MetricLabels, Counter>::default();
    pub static ref COUNT_KAFKA_PUBLISHED: Family<MetricLabels, Counter> =
        Family::<MetricLabels, Counter>::default();
    pub static ref MQTT_CONNECTED: Gauge = Gauge::default();
}

pub async fn init_metrics() {
    let mut registry = REGISTRY.lock().await;
    registry.register(
        "forwarding_mqtt_received",
        "Number of messages received from mqtt",
        COUNT_MQTT_RECEIVED.clone(),
    );
    registry.register(
        "forwarding_kafka_published",
        "Number of messages published to kafka",
        COUNT_KAFKA_PUBLISHED.clone(),
    );
    registry.register(
        "forwarding_mqtt_connected",
        "Is the connection to the MQTT broker active",
        MQTT_CONNECTED.clone(),
    );
    MQTT_CONNECTED.set(1); // During initialization MQTT is always connected otherwise it wouldn't get to this point
}

pub async fn metrics() -> String {
    let mut buffer = String::new();
    let registry = REGISTRY.lock().await;
    if let Err(err) = encode(&mut buffer, &registry) {
        log::error!("Could not encode metrics: {err}");
        // Dummy metric to signal a problem
        buffer = "forwarding_mqtt_connected 0\n".to_owned();
    }
    buffer
}
