use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub client_id: String,
    pub credentials: Option<MqttCredentials>,
    pub tls: Option<MqttTlsConfig>,
}

impl MqttConfig {
    pub fn clean_session(&self) -> bool {
        self.client_id.is_empty()
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct MqttTlsConfig {
    pub ca_cert: String,
    pub client_cert: Option<String>,
    pub client_key: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct MqttCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct KafkaConfig {
    pub bootstrap_server: String,
    pub port: u16,
    pub config: Option<HashMap<String,String>>,
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
    contents = parse_config(contents);
    serde_yaml::from_str(&contents).unwrap()
}

enum ParseState {
    NormalText,
    DollarSign,
    VarName(String),
}

fn parse_config(mut content: String) -> String {
    let mut vars: HashMap<String,Option<String>> = HashMap::new();
    let mut state = ParseState::NormalText;
    for char in content.chars() {
        state = match state {
            ParseState::NormalText => {
                if char == '$' {
                    ParseState::DollarSign
                } else {
                    ParseState::NormalText
                }
            },
            ParseState::DollarSign => {
                if char == '{' {
                    ParseState::VarName(String::new())
                } else {
                    ParseState::NormalText
                }
            },
            ParseState::VarName(mut buf) => {
                if char == '}' {
                    vars.insert(buf, None);
                    ParseState::NormalText
                } else if char == '\n' {
                    ParseState::NormalText
                } else {
                    buf.push(char);
                    ParseState::VarName(buf)
                }
            }
        }
    }
    for (envvar, value) in std::env::vars() {
        if vars.contains_key(&envvar) {
            vars.insert(envvar, Some(value));
        }
    }
    for (var, value) in vars {
        content = content.replace(format!("${{{var}}}").as_str(), value.unwrap_or_else(String::new).as_str());
    }
    content
}