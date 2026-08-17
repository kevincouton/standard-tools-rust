# syntax=docker/dockerfile:1
FROM rust:1.97.1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    libssl-dev \
    pkg-config \
    gfortran \
    make \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY proto ./proto

RUN cargo build --release -p sqt-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/standard-tools /usr/local/bin/standard-tools

EXPOSE 8080 50051

ENTRYPOINT ["/usr/local/bin/standard-tools"]
CMD ["server"]
