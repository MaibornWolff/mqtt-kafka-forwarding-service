use log::info;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

mod api;
mod config;
mod kafka;
mod metrics;
mod mqtt;

#[tokio::main(worker_threads = 8)]
async fn main() {
    env_logger::init();
    metrics::init_metrics().await;

    let running = Arc::new(AtomicBool::new(true));
    let config = config::load_config();
    let kafka_client = kafka::KafkaClient::new(&config.kafka).await;
    let mut mqtt_client =
        mqtt::MqttClient::new(&config.mqtt, config.forwarding, running.clone()).await;

    info!("Clients created. Subscribing to mqtt topics...");
    mqtt_client.subscribe().await;

    // Gracefully stop mqtt client on ctr-c
    let r = running.clone();
    ctrlc::set_handler(move || {
        info!("Stopping Mqtt Client...");
        r.store(false, Ordering::Release);
    })
    .expect("Error setting Crtl-C handler");

    info!("Starting HTTP API");
    tokio::task::spawn(api::api());

    info!("Running forwarding");
    mqtt_client.run(kafka_client, running).await;

    info!("Disconnecting...");
    mqtt_client.disconnect().await;
    info!("Stop.");
}
