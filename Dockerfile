FROM --platform=linux/arm64 rust:latest as builder

WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --bin camera

FROM --platform=linux/arm64 rust:latest
WORKDIR /app
COPY --from=builder /app/target/release/camera .
COPY certs ./certs
RUN mkdir -p /app/snapshots
CMD ["./camera"]