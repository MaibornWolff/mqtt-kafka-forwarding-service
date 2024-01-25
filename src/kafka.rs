use crate::config::KafkaConfig;
use log::error;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord, Producer};
use std::time::Duration;

#[derive(Clone)]
pub struct KafkaClient {
    producer: FutureProducer,
}

impl KafkaClient {
    pub async fn new(config: &KafkaConfig) -> KafkaClient {
        let mut client_config = ClientConfig::new();
        client_config
            .set("bootstrap.servers", config.url_string())
            .set("message.timeout.ms", "12000")
            .set("max.in.flight.requests.per.connection", "500");
        if let Some(params) = config.config.as_ref() {
            for (key, value) in params.iter() {
                client_config.set(key, value);
            }
        }
        let producer: FutureProducer = client_config
            .create()
            .expect("KafkaProducer creation error");
        // Check for connection
        match producer
            .client()
            .fetch_metadata(None, rdkafka::util::Timeout::After(Duration::from_secs(5)))
        {
            Ok(_) => (), // Got data, connection is established, nothing to do
            Err(err) => {
                panic!("Could not establish connection to kafka: {}", err);
            }
        }

        KafkaClient { producer }
    }

    pub fn in_flight_messages(&self) -> i32 {
        self.producer.in_flight_count()
    }

    pub async fn produce(&mut self, kafka_topic: &str, mqtt_topic: &str, payload: &[u8]) {
        for _ in 0..5 {
            let delivery_status = self
                .producer
                .send(
                    FutureRecord::to(kafka_topic)
                        .payload(payload)
                        .key(mqtt_topic),
                    Duration::from_secs(1),
                )
                .await;
            self.producer.poll(Duration::from_secs(0));
            match delivery_status {
                Ok((_partition, _offset)) => {
                    return;
                }
                Err((err, _msg)) => {
                    error!("Failed to send: {}", err);
                }
            };
        }
        // If we come here we failed to send the message
        panic!("Could not send a message. Aborting")
    }
}
