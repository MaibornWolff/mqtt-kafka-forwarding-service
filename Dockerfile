FROM clux/muslrust:1.75.0 as builder
RUN mkdir /build
WORKDIR /build
COPY Cargo.toml Cargo.lock /build/
COPY src /build/src
COPY benchmark /build/benchmark
ENV CARGO_HOME=/build/.cargo
RUN --mount=type=cache,target=/build/target --mount=type=cache,target=/build/.cargo \
    cargo build --release && \
    cp /build/target/x86_64-unknown-linux-musl/release/forwarder /build/forwarder


FROM alpine:3.18
RUN mkdir /forwarding-service
WORKDIR /forwarding-service
COPY --from=builder /build/forwarder /forwarding-service/forwarder
CMD ["/forwarding-service/forwarder"]
