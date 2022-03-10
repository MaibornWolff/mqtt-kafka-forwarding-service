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
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", config.url_string())
            .set("message.timeout.ms", "12000")
            .set("max.in.flight.requests.per.connection", "500")
            .create()
            .expect("KafkaProducer creation error");
        KafkaClient { producer }
    }

    pub fn in_flight_messages(&self) -> i32 {
        self.producer.in_flight_count()
    }

    pub async fn produce(&mut self, topic: &String, payload: &[u8]) {
        for _ in 0..5 {
            let delivery_status = self
            .producer
            .send(
                FutureRecord::to(topic).payload(payload).key(&String::new()),
                Duration::from_secs(1),
            )
            .await;
            self.producer.poll(Duration::from_secs(0));
            match delivery_status {
                Ok((_partition, _offset)) => {
                    return;
                },
                Err((err, _msg)) => {
                    error!("Failed to send: {}", err);
                }
            };
        }
        // If we come here we failed to send the message
        panic!("Could not send a message. Aborting")
    }
}
