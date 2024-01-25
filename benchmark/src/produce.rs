use crate::message::BenchmarkMessage;
use crate::MQTT_TOPIC;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::time::Instant;

pub struct BenchmarkProducer {
    messages: Vec<BenchmarkMessage>,
    client: AsyncClient,
}

pub fn map_qos(qos: Option<u8>) -> rumqttc::QoS {
    qos.map_or_else(
        || rumqttc::QoS::AtLeastOnce,
        |qos| rumqttc::qos(qos).expect("Invalid qos"),
    )
}

impl BenchmarkProducer {
    pub fn new(stop_signal: Arc<AtomicBool>) -> BenchmarkProducer {
        let messages = Vec::<BenchmarkMessage>::new();
        let mut mqttoptions = MqttOptions::new("stresstest-producer", "localhost", 1883);
        mqttoptions
            .set_clean_session(true)
            .set_inflight(10)
            .set_manual_acks(false);

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
        tokio::spawn(async move {
            while !stop_signal.load(Ordering::Relaxed) {
                match eventloop.poll().await {
                    Ok(_) => (),
                    Err(err) => println!("Error in MQTT Event queue: {:?}", err),
                }
            }
        });
        BenchmarkProducer { messages, client }
    }

    pub async fn run(&mut self, qos: QoS, n_msg: usize, rate_limit: Option<u32>) -> Duration {
        let wait_time = match rate_limit {
            None => Duration::from_nanos(0u64),
            Some(rate_limit) => Duration::from_nanos((1e9 as u64) / (rate_limit as u64)),
        };
        let mut last_msg = Instant::now();
        let time_start = Instant::now();
        for count in 0..n_msg {
            while rate_limit.is_some() && last_msg.elapsed() < wait_time {
                tokio::task::yield_now().await;
            }
            last_msg = Instant::now();
            let message = BenchmarkMessage {
                id: count as u64,
                last: count == n_msg - 1,
            };
            let payload_string = serde_json::to_string(&message).unwrap();
            let payload: &[u8] = payload_string.as_ref();
            match self.client.publish(MQTT_TOPIC, qos, true, payload).await {
                Ok(_) => {
                    self.messages.push(message);
                }
                Err(response) => {
                    println!("Error while publishing: {:?}", response);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            };
        }
        time_start.elapsed()
    }

    pub async fn stop(&mut self) {
        match self.client.disconnect().await {
            Ok(_) => (),
            Err(response) => println!("Error while disconnecting: {:?}", response),
        }
    }
}
