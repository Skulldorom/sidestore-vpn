#!/bin/sh
set -e

# Start the sidestore VPN process in background
/sidestore-vpn "$@" &
SIDESTORE_PID=$!

# Start tailscaled in background
echo "boot: $(date '+%Y/%m/%d %H:%M:%S') Starting tailscaled"
tailscaled \
    --socket=/tmp/tailscaled.sock \
    --statedir=/var/lib/tailscale \
    --state=/var/lib/tailscale/tailscaled.state \
    &
TAILSCALED_PID=$!

cleanup() {
    kill "$SIDESTORE_PID" 2>/dev/null || true
    kill "$TAILSCALED_PID" 2>/dev/null || true
}

# Wait for tailscaled socket
echo "boot: $(date '+%Y/%m/%d %H:%M:%S') Waiting for tailscaled socket at /tmp/tailscaled.sock"
wait_seconds=0
while [ ! -S /tmp/tailscaled.sock ]; do
    wait_seconds=$((wait_seconds + 1))
    if [ "$wait_seconds" -ge 30 ]; then
        echo "boot: $(date '+%Y/%m/%d %H:%M:%S') tailscaled socket not available after 30 seconds"
        cleanup
        exit 1
    fi
    sleep 1
done

# Build tailscale up arguments
UP_ARGS=""
[ -n "${TS_AUTHKEY}" ]              && UP_ARGS="${UP_ARGS} --authkey=${TS_AUTHKEY}"
[ -n "${TS_ROUTES}" ]               && UP_ARGS="${UP_ARGS} --advertise-routes=${TS_ROUTES}"
[ -n "${TS_HOSTNAME}" ]             && UP_ARGS="${UP_ARGS} --hostname=${TS_HOSTNAME}"
[ -n "${TS_EXTRA_ARGS}" ]           && UP_ARGS="${UP_ARGS} ${TS_EXTRA_ARGS}"
[ -n "${TS_TAILSCALE_UP_EXTRA_ARGS}" ] && UP_ARGS="${UP_ARGS} ${TS_TAILSCALE_UP_EXTRA_ARGS}"

# Run tailscale up
echo "boot: $(date '+%Y/%m/%d %H:%M:%S') Running 'tailscale up'"
# shellcheck disable=SC2086
if ! tailscale --socket=/tmp/tailscaled.sock up ${UP_ARGS}; then
    echo "boot: $(date '+%Y/%m/%d %H:%M:%S') failed to auth tailscale"
    cleanup
    exit 1
fi

echo "boot: $(date '+%Y/%m/%d %H:%M:%S') Tailscale is up"

# Keep running until tailscaled exits
wait "$TAILSCALED_PID"
