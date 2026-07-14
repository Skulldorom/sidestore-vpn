# Build stage
FROM rust:alpine AS builder

# Install musl-dev for static linking
RUN apk add --no-cache musl-dev

WORKDIR /app

# Copy the source code
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Run the full test suite in the release profile before producing the binary.
# This lets the following release build reuse compiled dependencies instead of
# compiling the same crates once for dev tests and again for release.
RUN cargo test --release --locked

# Build a statically linked binary
RUN cargo build --release --locked

# Final stage
FROM scratch

# Copy the statically linked binary
COPY --from=builder /app/target/release/sidestore-vpn /sidestore-vpn

# Docker sends SIGTERM by default, but sidestore-vpn already exits cleanly on
# SIGINT through its ctrlc handler.
STOPSIGNAL SIGINT

# Set the entrypoint
ENTRYPOINT ["/sidestore-vpn"]
