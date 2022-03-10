FROM alpine:3.15

RUN mkdir /forwarding-service
WORKDIR /forwarding-service
COPY target/x86_64-unknown-linux-musl/release/forwarder /forwarding-service/forwarder

CMD ["/forwarding-service/forwarder"]
