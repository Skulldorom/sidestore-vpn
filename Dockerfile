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

# Final stage - base image from scratch
FROM scratch AS base

# Copy the statically linked binary
COPY --from=builder /app/target/release/sidestore-vpn /sidestore-vpn

# Set the entrypoint
ENTRYPOINT ["/sidestore-vpn"]

# Tailscale image
FROM tailscale/tailscale:stable AS tailscale
ENV TS_ROUTES="10.7.0.1/32"
ENV TS_USERSPACE="false"
ENV TS_EXTRA_ARGS="--snat-subnet-routes=false"
ENV TS_TAILSCALED_EXTRA_ARGS="--verbose=-1"
COPY --from=base /sidestore-vpn /sidestore-vpn
COPY --chmod=755 tailscale-entrypoint.sh /tailscale-entrypoint.sh
ENTRYPOINT ["/tailscale-entrypoint.sh"]

FROM base
