use std::time::Duration;

use crate::message::BenchmarkMessage;
use base64::prelude::*;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    ClientConfig, Message, Offset, TopicPartitionList,
};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

static TOPIC_NAME: &str = "stresstest";

pub struct BenchmarkConsumer {
    client: StreamConsumer,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WrappedPayload {
    topic: String,
    payload: String,
}

impl BenchmarkConsumer {
    pub fn new() -> BenchmarkConsumer {
        let client = ClientConfig::new()
            .set("bootstrap.servers", "localhost")
            .set("group.id", "stresstest_benchmark")
            .create()
            .expect("Consumer creation error");
        BenchmarkConsumer { client }
    }
    pub async fn subscribe(&mut self) {
        let topics = [TOPIC_NAME];
        self.client
            .subscribe(&topics)
            .expect("Error subscribing to kafka topic");
        // Manually assign partitions to not wait for automatic rebalance
        let mut topiclist = TopicPartitionList::new();
        topiclist
            .add_partition_offset(TOPIC_NAME, 0, Offset::Beginning)
            .unwrap();
        self.client.assign(&topiclist).unwrap();
        // Create stream to force assignment to complete to not lose messages
        let _ = self.client.stream();
    }
    pub async fn run(&mut self, wrapped_payload: bool) -> (Duration, Vec<BenchmarkMessage>) {
        let mut messages = Vec::new();
        let mut count = 0u64;
        let mut start_instant = Instant::now();
        loop {
            match self.client.recv().await {
                Ok(msg) => {
                    count += 1;
                    if count == 1 {
                        start_instant = Instant::now();
                    }
                    let message: BenchmarkMessage =
                        unwrap_message(msg.payload().unwrap(), wrapped_payload);
                    let last = message.last;
                    if count == 1 && message.id != 0 {
                        println!("WARNING: First received message does have ID {} instead of 0. Message might be missing.\n", message.id);
                    }
                    messages.push(message);
                    if last {
                        break;
                    }
                }
                Err(err) => println!("Error while receiving a message: {:?}", err),
            }
        }
        let stop_instant = Instant::now();
        (stop_instant - start_instant, messages)
    }
}

fn unwrap_message(payload: &[u8], wrapped_payload: bool) -> BenchmarkMessage {
    if wrapped_payload {
        let msg: WrappedPayload = serde_json::from_slice(payload).unwrap();
        let payload = BASE64_STANDARD.decode(msg.payload).unwrap();
        serde_json::from_slice(payload.as_ref()).unwrap()
    } else {
        serde_json::from_slice(payload).unwrap()
    }
}
