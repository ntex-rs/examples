FROM rust:1 as builder
WORKDIR /app
ADD . /app
RUN rustup target add x86_64-unknown-linux-musl
RUN CARGO_HTTP_MULTIPLEXING=false cargo build --release --target x86_64-unknown-linux-musl

FROM scratch
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/docker_sample /app
EXPOSE 5000
CMD ["/app"]
