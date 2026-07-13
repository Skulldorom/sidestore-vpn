# SideStore VPN tool

[SideStore](https://github.com/SideStore/SideStore) usually requires a WireGuard tunnel or [StosVPN](https://github.com/SideStore/StosVPN), so traffic generated during app install/refresh process can be hijacked and processed by SideStore itself.

This tool provides an alternative to the aforementioned tools, and can allow SideStore to work across all iOS devices on your local network, without setting up WireGuard or StosVPN individually.

# How it works

When installing or refreshing apps, SideStore opens ports on local iOS device mimicing a computer running developer software. Then it instructs iOS to connect to a computer at `10.7.0.1`.

This tool creates a TUN device expecting packets to 10.7.0.1, swap the source/destination field of each packet, and send them so that they are forwarded back to the iOS device sending the request. This will get iOS talking with SideStore's fake computer, and allow apps to be installed/refreshed.

This is the same approach as used by StosVPN: <https://github.com/SideStore/StosVPN/blob/main/TunnelProv/PacketTunnelProvider.swift>

## Docker with Tailscale

Two Docker images are available:

- **`ghcr.io/skulldorom/sidestore-vpn`** — the sidestore-vpn binary (minimal scratch image, ~2 MB)
- **`tailscale/tailscale:stable`** — official Tailscale image (handles mesh networking and route advertisement)

The recommended setup runs both containers together. The sidestore-vpn container shares the Tailscale container's network namespace via `network_mode: "service:tailscale"`, so all traffic routed through Tailscale reaches the TUN device created by sidestore-vpn.

```bash
# Create a state directory for Tailscale persistence
mkdir -p state

# Start both containers with docker compose
docker compose up -d
```

Create a `.env` file with your Tailscale auth key and optional settings:

```bash
TS_AUTHKEY=tskey-xxxxxxx
TS_HOSTNAME=sidestore-vpn  # optional, defaults to sidestore-vpn
```

## Docker Compose with Tailscale

The `docker-compose.yml` file uses the following environment variables:

| Variable        | Description                            | Default                      |
| --------------- | -------------------------------------- | ---------------------------- |
| `TS_AUTHKEY`    | Tailscale auth key (required)          | —                            |
| `TS_HOSTNAME`   | Hostname shown in Tailscale admin      | `sidestore-vpn`              |
| `TS_ROUTES`     | Subnet routes advertised via Tailscale | `10.7.0.1/32`                |
| `TS_EXTRA_ARGS` | Extra arguments passed to Tailscale    | `--snat-subnet-routes=false` |

The Tailscale state is persisted in `./state` so you don't need to re-authenticate on restart.

Both containers require the `/dev/net/tun` device and the `NET_ADMIN` capability — Tailscale for its own TUN interface, and sidestore-vpn for its packet-rewriting TUN device.

# Credit

Thanks to [SideStore](https://github.com/SideStore/SideStore) for creating an app to easily install apps on iOS devices.

Thanks to [StosVPN](https://github.com/SideStore/StosVPN) for the approach in networking.

# License

Public domain.
