use crate::config::{ForwardingConfig, MqttConfig, MqttTlsConfig};
use crate::kafka::KafkaClient;
use log::info;
use rumqttc::{
    matches, AsyncClient, Event, EventLoop, Key, MqttOptions, Packet, Publish, QoS,
    SubscribeFilter, TlsConfiguration, Transport,
};
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

static MAX_IN_FLIGHT: u16 = 10;

#[derive(Clone, Debug)]
struct TopicMatch {
    mqtt_topic: String,
    kafka_topic: String,
    wrap_as_json: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WrappedPayload {
    topic: String,
    payload: String,
}

pub struct MqttClient {
    client: AsyncClient,
    eventloop: EventLoop,
    stats: Arc<Stats>,
    topic_config: Vec<TopicMatch>,
}

struct Stats {
    pub count_received: AtomicU64,
    pub count_published: AtomicU64,
    pub in_flight: AtomicI32,
}

impl Stats {
    fn new() -> Stats {
        let count_received = AtomicU64::new(0);
        let count_published = AtomicU64::new(0);
        let in_flight = AtomicI32::new(0);
        Stats {
            count_received,
            count_published,
            in_flight,
        }
    }
}

fn init_tls_transport(config: MqttTlsConfig) -> Transport {
    let ca_cert = std::fs::read_to_string(&config.ca_cert).expect("Could not read CA cert file");
    let client_auth = if config.client_cert.is_some() && config.client_key.is_some() {
        let client_cert =
           std::fs::read_to_string(config.client_cert.unwrap()).expect("Could not read client cert");
        let client_key =
            std::fs::read_to_string(config.client_key.unwrap()).expect("Could not read client key");
        Some((client_cert.into_bytes(), Key::RSA(client_key.into_bytes())))
    } else {
        None
    };
    Transport::Tls(TlsConfiguration::Simple {
        ca: ca_cert.into_bytes(),
        alpn: None,
        client_auth,
    })
}

impl MqttClient {
    pub async fn new(
        config: &MqttConfig,
        forwardings: Vec<ForwardingConfig>,
        running: Arc<AtomicBool>,
    ) -> MqttClient {
        let mut mqttoptions =
            MqttOptions::new(config.client_id.clone(), config.host.clone(), config.port);
        mqttoptions
            .set_clean_session(config.clean_session())
            .set_inflight(MAX_IN_FLIGHT)
            .set_manual_acks(true);

        if let Some(tlsconfig) = config.tls.as_ref() {
            log::debug!("Using TLS for MQTT connection");
            mqttoptions.set_transport(init_tls_transport(tlsconfig.clone()));
        }
        if let Some(credentials) = config.credentials.as_ref() {
            mqttoptions.set_credentials(&credentials.username, &credentials.password);
        }

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, MAX_IN_FLIGHT as usize);

        // Do one poll to check if the connection is established
        match eventloop.poll().await {
            Ok(_packet) => {},
            Err(err) => {
                panic!("Failed to connect to mqtt: {}", err);
            }
        }

        let topic_config = forwardings
            .iter()
            .map(|forwarding_config| TopicMatch {
                mqtt_topic: forwarding_config.mqtt.topic.clone(),
                kafka_topic: forwarding_config.kafka.topic.clone(),
                wrap_as_json: forwarding_config.wrap_as_json.unwrap_or(false),
            })
            .collect::<Vec<TopicMatch>>();

        let stats = Arc::new(Stats::new());
        let s = stats.clone();
        tokio::spawn(async move {
            stats_reporter(running, s).await;
        });

        MqttClient {
            client,
            eventloop,
            stats,
            topic_config,
        }
    }

    pub async fn subscribe(&mut self) {
        let subscribe_filter = self.topic_config.iter().map(|topic_match| {
            SubscribeFilter::new(topic_match.mqtt_topic.clone(), QoS::ExactlyOnce)
        });
        self.client
            .subscribe_many(subscribe_filter)
            .await
            .expect("Error while subscribing to mqtt topics");
    }

    pub async fn run(&mut self, kafka: KafkaClient, running: Arc<AtomicBool>) {
        while running.load(Ordering::Relaxed) {
            tokio::select! {
                Ok(event) = self.eventloop.poll() => {
                    match event {
                        Event::Incoming(Packet::Publish(publish)) => {
                            self.handle_publish(&kafka, publish).await;
                        },
                        Event::Incoming(Packet::SubAck(_)) => {
                            info!("Subscribed to MQTT topics successfully");
                        },
                        _ => (),
                    }
                },
                _ = tokio::time::sleep(Duration::from_secs(2)) => (),
            }
        }
    }

    async fn handle_publish(&mut self, kafka: &KafkaClient, publish: Publish) {
        self.stats.count_received.fetch_add(1, Ordering::Relaxed);
        let kafka_topics = matching_topics(&publish.topic, &self.topic_config);

        // Wait for in_flight messages to be low enough
        while kafka.in_flight_messages() >= 1000 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        self.stats
            .in_flight
            .store(kafka.in_flight_messages(), Ordering::Relaxed);

        // Spawn new thread for each mqtt message to not block the eventloop
        let mqtt_client = self.client.clone();
        let mut kafka_client = kafka.clone();
        let stats = self.stats.clone();
        tokio::spawn(async move {
            let wrapped_payload = wrap_payload(&publish);
            let payload = publish.payload.as_ref();
            for topic in kafka_topics {
                if topic.wrap_as_json {
                    kafka_client
                        .produce(&topic.kafka_topic, wrapped_payload.as_ref())
                        .await;
                } else {
                    kafka_client.produce(&topic.kafka_topic, payload).await;
                }
                stats.count_published.fetch_add(1, Ordering::Relaxed);
            }
            for _ in 0..5 {
                if mqtt_client.ack(&publish).await.is_ok() {
                    return;
                }
            }
            panic!("Could not send ack to MQTT. Aborting");
        });
    }

    pub async fn disconnect(&mut self) {
        self.client.disconnect().await.unwrap();
    }
}

fn matching_topics(mqtt_topic: &str, topic_config: &[TopicMatch]) -> Vec<TopicMatch> {
    topic_config
        .iter()
        .filter(|topic_match| matches(mqtt_topic, &topic_match.mqtt_topic))
        .cloned()
        //.map(|topic_match| topic_match.clone())
        .collect::<Vec<TopicMatch>>()
}

async fn stats_reporter(running: Arc<AtomicBool>, stats: Arc<Stats>) {
    let mut last = 0;
    while running.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let current = stats.count_received.load(Ordering::Relaxed);
        let current_published = stats.count_published.load(Ordering::Relaxed);
        if current == 0 && last == 0 {
            continue;
        }
        let in_flight = stats.in_flight.load(Ordering::Relaxed);
        info!(
            " > Stats > received: {} published: {} (~{} msg/s) in flight: {}",
            current,
            current_published,
            ((current - last) / 2),
            in_flight
        );
        if current == last && current != 0 {
            info!("resetting counter because no new messages were received.");
            last = 0;
            stats.count_received.store(0, Ordering::Relaxed);
            stats.count_published.store(0, Ordering::Relaxed);
        } else {
            last = current;
        }
    }
}

fn wrap_payload(publish: &Publish) -> Vec<u8> {
    let payload = base64::encode(publish.payload.clone());
    let obj = WrappedPayload {
        topic: publish.topic.clone(),
        payload,
    };
    serde_json::to_vec(&obj).unwrap()
}
