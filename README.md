# MQTT-Kafka-Forwarding-Service

A service that forwards messages from an MQTT broker to Kafka based on configurable topic mappings.

The service is written in Rust and has the following features:

* Guarantees At-Least-Once operations due to using manual acknowledgement in MQTT
* Optionally wraps MQTT payloads in a json object which preserves the original topic (`{"topic": "foo/bar", "payload": "somebase64edpayload"}`). Can be useful if later processing steps need the original MQTT topic (e.g. if some device-id is encoded in the topic but not repeated in the payload)

## Quickstart

The forwarding-service can be deployed in kubernetes using our helm chart:

1. `helm repo add maibornwolff https://maibornwolff.github.io/mqtt-kafka-forwarding-service/`
2. Create a values.yaml file with your configuration:

      ```yaml
      fullNameOverride: mqtt-kafka-forwarding-service
      config: |  # Service configuration (see section "Configuration" below for details)
        mqtt:
          host: mqtt.default.svc.cluster.local  # DNS name of your mqtt broker
          port: 1883
          client_id: 'kafka-forwarding-service-1'
        kafka:
          bootstrap_server: kafka.default.svc.cluster.local  # DNS name of your kafka broker
          port: 9092
        forwarding:
          - name: foobar
            mqtt:
              topic: foo/bar/#
            kafka:
              topic: foobar
            wrap_as_json: false
      ```

3. `helm install mqtt-kafka-forwarding-service maibornwolff/mqtt-kafka-forwarding-service -f values.yaml`

Or if you are running outside of Kubernetes the forwarding-service can be deployed as a docker image:

1. Copy `config.yaml` to `my-config.yaml` and adapt it to your needs
2. Run `docker run -v $(pwd)/my-config.yaml:/forwarding-service/config.yaml ghcr.io/maibornwolff/mqtt-kafka-forwarding-service:0.1.0`

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
  tls: # Optional, TLS-related config
    ca_cert: # Path to a PEM-encoded cert to verify the presented broker certificate against, must be supplied if tls is set
    client_key: # Path to a PEM-encoded client key to use for client authentication, optional, if set client_cert must also be set
    client_cert: # Path to a PEM-encoded client cert to use for client authentication, optional, if set client_key mut also be set
  credentials: # Optional
    username: # Username to use for authentication
    password: # Password to use for authentication
kafka:
  bootstrap_server: localhost # Host/DNS name of the kafka server
  port: 9092 # Port of the kafk server
  config: {}  # Key-Value pairs of extra config to supply to the Kafka Producer
forwarding: # List of forwardings
  - name: demo # A unique name
    mqtt:
      topic: 'demo/#' # MQTT topic to subscribe to, can be a wildcard
    kafka:
      topic: demo_data # Kafka topic to send data to
    wrap_as_json: false # Should the payload be wrapped in a json object, optional, defaults to false
```

Under `kafka.config` you can specify further options for the kafka producer (e.g. to configure SSL or authentication). This service uses [librdkafka](https://github.com/edenhill/librdkafka) so check its [CONFIGURATION.md](https://github.com/edenhill/librdkafka/blob/master/CONFIGURATION.md) for all possible configuration options.
To securely provide sensitive information (e.g. a password) you can use environment variables in the config, by specifying `${ENVIRONMENT_VARIABLE}`. Note that this is not injection-safe in any way, so use it only with variables you control.

By default the service will read the configuration from a file called `config.yaml` from the working directory. To use a different file set the environment variable `CONFIG_FILE` to its path.

### TLS

The forwarding-service can be configured to use TLS/SSL for both MQTT and Kafka connections. For both procotols you need the PEM-encoded CA certificate that has signed the server certificate and, if you want to do client certificate authentication, the PEM-encoded client certificate and key (for MQTT it has to be an RSA key).

For MQTT you need to specify the following configuration:

```yaml
mqtt:
  # ...
  tls:
    ca_cert: mqtt-ca.cert.pem
    client_key: mqtt-client.cert.pem # Optional, only if client cert authentication is needed
    client_cert: mqtt-client.key.pem # Optional, only if client cert authentication is needed
```

For Kafka you need to specifiy the following configuration:

```yaml
kafka:
    # ...
    config:
      security.protocol: ssl
      ssl.ca.location: kafka-ca.cert.pem
      ssl.certificate.location: kafka-client.cert.pem # Optional, only if client cert authentication is needed
      ssl.key.location: kafka-client.cert.pem # Optional, only if client cert authentication is needed
```

### Authentication

The forwarding-service supports two ways for the service to authenticate itsself to MQTT and Kafka:

Client certificate authentication: See [TLS](#TLS) above.

Username/Password authentication:

```yaml
mqtt:
  # ...
  credentials:
    username: forwardinguser
    password: supersecretpassword
kafka:
    # ...
    config:
      security.protocol: SASL_SSL
      sasl.mechanism: PLAIN
      ssl.ca.location: kafka/ca.cert.pem
      sasl.username: foobar
      sasl.password: foopassword
```

Note that SASL GSSAPI is currently not supported out-of-the-box. If you need it, add the `gssapi` feature to the `rdkafka` dependency in `Cargo.toml` and rebuild. In this case you also need to provide custom docker build and base images that contain `libsasl`.

## Benchmark

This repository includes a benchmark tool to measure the throughput of the forwarding service. The benchmark will publish a number of messages to an MQTT topic and then measure how long it takes for the messages to arrive on the corresponding kafka topic.

To run it execute the following steps:

1. Build the forwarding-service and the benchmark: `cargo build --release --all`
2. Start the MQTT broker: `docker run -d --name hivemq -p 1883:1883 -v $(pwd)/benchmark/hivemq-config.xml:/opt/hivemq/conf/config.xml hivemq/hivemq-ce:latest`
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

Enabling the payload wrapping does not have any measurable impact on performance. Also HiveMQ is not the bottleneck, using QoS 1 the benchmark tool is able to publish about 9000 msg/s to the broker, so way more than the forwarding-service can process. Measuring the performance for QoS 0 was hard as at higher message rates messages get dropped (this is allowed in the specification and can happen in the broker or in the libraries on either the publisher or subscriber side due to overload). As such the provided number is one where no message drops happened. In some measurements about 10000 msg/s were possible without suffering losses, going higher increases the risk of dropped messages exponentially.

We also created a Python implementation of the forwarding service (sourcecode not included in this repository) to compare performance and because python is more common in the company than Rust. We implemented two variants, one using the [confluent-kafka-python library](https://github.com/confluentinc/confluent-kafka-python) and one using the [kafka-python library](https://github.com/dpkp/kafka-python). At first glance the confluent library showed good peak performance of about 1200 msg/s for QoS 1. But when running the benchmark with more messages or severl times then quite quickly the message rate drops significantly to about 300 msg/s. A flamegraph analysis showed that most of the time is spent in kafka library code (communicating with and waiting for the broker), so we assume the performance drop is due to some behaviour of the library. Switching out the confluent library with the third-pary kafka-python library gives a stable performance but slower than the confluent library peak performance.

| QoS | confluent peak rate | confluent avg rate | kafka-python avg rate |
|-----|---------------------|--------------------|-----------------------|
| 0   |  2000 msg/s         | 350 msg/s          | 1000 msg/s            |
| 1   |  1200 msg/s         | 300 msg/s          |  750 msg/s            |
| 2   |  1000 msg/s         | 250 msg/s          |  600 msg/s            |

Before we started the comparison we expected the python implementation to be slower than the rust variant due to python being an interpreted language. But we also assumed the performance drop would not be that much as the forwarding-service should be primarily I/O-bound so the slower interpreter performance should not have too much of an impact. The average performance of the confluent library was a bit of a disappointment as its peak performance was in line with our expectations. Considering kafka-python is a pure python library its performance is quite good.
