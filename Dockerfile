# Build stage
FROM rust:alpine AS builder

# Install musl-dev for static linking
RUN apk add --no-cache musl-dev

WORKDIR /app

# Copy the source code
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build a statically linked binary
RUN cargo build --release

# Final stage
FROM scratch

# Copy the statically linked binary
COPY --from=builder /app/target/release/sidestore-vpn /sidestore-vpn

# Set the entrypoint
ENTRYPOINT ["/sidestore-vpn"]
