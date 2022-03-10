# MQTT-Kafka-Forwarding-Service

A service that forwards messages from an MQTT broker to Kafka based on configurable topic mappings.

The service is written in Rust and has the following features:

* Guarantees At-Least-Once operations due to using manual acknowledgement in MQTT
* Optionally wraps MQTT payloads in a json object which preserves the original topic (`{"topic": "foo/bar", "payload": "somebase64edpayload"}`)

## Quickstart

The forwarding-service can be deployed as a docker image:

1. Copy `config.yaml` to `my-config.yaml` and adapt it to your needs
2. Run `docker run -v $(pwd)/my-config.yaml:/forwarding-service/config.yaml ghcr.io/maibornwolff/mqtt-kafka-forwarding-service:latest`

## Custom build

You can build your own custom binary by following these steps:

1. Check out this repository
2. Run `cargo build --release`
3. The binary can be found under `target/release/forwarder`

To build your own docker image follow these steps:

1. `docker run --rm -it -v cargo-cache:/root/.cargo/registry -v "$(pwd)":/volume clux/muslrust:1.59.0 cargo build --release`
2. `docker build . -t <my-image-name>`

## Configuration

The service can be configured via a yaml config file with the following structure:

```yaml
mqtt:
    host: localhost # Host/DNS name of the MQTT broker
    port: 1883 # Port of the MQTT broker
    client_id: 'forwarding-service-1' # Client-ID to use, if not specified a clean session will be used
kafka:
    bootstrap_server: localhost # Host/DNS name of the kafka server
    port: 9092 # Port of the kafk server
forwarding: # List of forwardings
    - name: demo # A unique name
      mqtt:
        topic: 'demo/#' # MQTT topic to subscribe to, can be a wildcard
      kafka:
        topic: demo_data # Kafka topic to send data to
      wrap_as_json: false # Should the payload be wrapped in a json object
```

By default the service will read the configuration from a file called `config.yaml` from the working directory. To use a different file set the environment variable `CONFIG_FILE` to its path.

## Benchmark

This repository includes a benchmark tool to measure the throughput of the forwarding service. The benchmark will publish a number of messages to an MQTT topic and then measure how long it takes for the messages to arrive on the corresponding kafka topic.

To run it execute the following steps:

1. Build the forwarding-service and the benchmark: `cargo build --release --all`
2. Start the MQTT broker: `docker run -d --name benchmark-hivemq -p 1883:1883 -v $(pwd)/benchmark/hivemq-config.xml:/opt/hivemq/conf/config.xml hivemq/hivemq-ce:latest`
3. Start Kafka: `docker run -d --name kafka -p 9092:9092 bashj79/kafka-kraft`
4. Start the forwarding service: `CONFIG_FILE=benchmark/config.yaml ./target/release/forwarder`
5. Start the benchmark (in another terminal): `./target/release/benchmark --messages 100000`

If you want you can adjust the behaviour of the benchmark with the following options:

* `--qos` to change the QoS from its default of `1` (At-Least-Once)
* `--msg-rate` to limit the number of messages per second to publish to MQTT (if the MQTT broker has a limited queue capacity)
* `--wrapped-payload` / `-w` if `wrap_as_json` is set to true in the forwarding config

## Performance

Performance was measured using the benchmark tool by repeatedly sending 1 million messages (sending message rate is limited to 10000 to not overwhelm the MQTT broker queues). All rates are averaged and rounded over several runs.

| QoS | Message rate |
|-----|--------------|
| 0   | 8750 msg/s   |
| 1   | 2000 msg/s   |
| 2   | 1900 msg/s   |

Enabling the payload wrapping does not have any measurable impact on performance.
