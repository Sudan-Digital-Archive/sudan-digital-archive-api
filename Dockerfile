FROM rust:1.84.1-slim-bullseye AS builder

WORKDIR /opt
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY src src/
COPY entity entity/
COPY migration migration/
RUN cargo build --release

FROM debian:bullseye-slim

WORKDIR /opt
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/* && \
    groupadd -r ferris && useradd -r -g ferris ferris
COPY --from=builder --chown=ferris:ferris \
    /opt/target/release/sudan-digital-archive-api ./
USER ferris
CMD ["./sudan-digital-archive-api"]