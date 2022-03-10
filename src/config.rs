use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub client_id: String,
}

impl MqttConfig {
    pub fn clean_session(&self) -> bool {
        return self.client_id.is_empty();
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct KafkaConfig {
    pub bootstrap_server: String,
    pub port: u16,
}

impl KafkaConfig {
    pub fn url_string(&self) -> String {
        return format!("{}:{}", self.bootstrap_server, self.port);
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ForwardingConfig {
    pub name: String,
    pub mqtt: MqttSource,
    pub kafka: KafkaDest,
    pub wrap_as_json: Option<bool>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MqttSource {
    pub topic: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct KafkaDest {
    pub topic: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub mqtt: MqttConfig,
    pub kafka: KafkaConfig,
    pub forwarding: Vec<ForwardingConfig>,
}

pub fn load_config() -> Config {
    let path = std::env::var("CONFIG_FILE").unwrap_or_else(|_| "config.yaml".to_string());
    let mut f = File::open(path).expect("config file not found");
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("something went wrong reading the config file");
    serde_yaml::from_str(&contents).unwrap()
}
