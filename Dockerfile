# syntax=docker/dockerfile:1.7

# Build stage
FROM rust:alpine AS builder

# Install musl-dev for static linking
RUN apk add --no-cache musl-dev

WORKDIR /app

# Copy the source code
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Run tests and build in the release profile so dependencies are compiled once.
# BuildKit cache mounts keep the Cargo registry and target dir warm across CI
# runs even when source changes invalidate this layer. Small mercy, still CI.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo test --release --locked && \
    cargo build --release --locked && \
    mkdir -p /out && \
    cp target/release/sidestore-vpn /out/sidestore-vpn

# Final stage
FROM scratch

# Copy the statically linked binary
COPY --from=builder /out/sidestore-vpn /sidestore-vpn

# Docker sends SIGTERM by default, but sidestore-vpn already exits cleanly on
# SIGINT through its ctrlc handler.
STOPSIGNAL SIGINT

# Set the entrypoint
ENTRYPOINT ["/sidestore-vpn"]
