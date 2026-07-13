# SideStore VPN tool

[SideStore](https://github.com/SideStore/SideStore) usually requires a WireGuard tunnel or [StosVPN](https://github.com/SideStore/StosVPN), so traffic generated during app install/refresh process can be hijacked and processed by SideStore itself.

This tool provides an alternative to the aforementioned tools, and can allow SideStore to work across all iOS devices on your local network, without setting up WireGuard or StosVPN individually.

# How it works

When installing or refreshing apps, SideStore opens ports on local iOS device mimicing a computer running developer software. Then it instructs iOS to connect to a computer at `10.7.0.1`.

This tool creates a TUN device expecting packets to 10.7.0.1, swap the source/destination field of each packet, and send them so that they are forwarded back to the iOS device sending the request. This will get iOS talking with SideStore's fake computer, and allow apps to be installed/refreshed.

This is the same approach as used by StosVPN: <https://github.com/SideStore/StosVPN/blob/main/TunnelProv/PacketTunnelProvider.swift>

## Docker Compose with Tailscale

The Docker image at `ghcr.io/Skulldorom/sidestore-vpn` contains only the Rust binary — Tailscale runs as a separate service via the official `tailscale/tailscale:stable` image. This keeps Tailscale decoupled from the build: updates to the VPN binary don't force a Tailscale re-download, and Tailscale updates are a simple `docker compose pull`.

Create a `.env` file with your Tailscale auth key and optionally a custom hostname:

```bash
TS_AUTHKEY=tskey-xxxxxxx
TS_HOSTNAME=sidestore-vpn  # optional, defaults to sidestore-vpn
```

Then start both services:

```bash
docker compose up -d
```

Environment variables used by the Compose file:

| Variable        | Description                            | Default                      |
| --------------- | -------------------------------------- | ---------------------------- |
| `TS_AUTHKEY`    | Tailscale auth key (required)          | —                            |
| `TS_HOSTNAME`   | Hostname shown in Tailscale admin      | `sidestore-vpn`              |
| `TS_ROUTES`     | Subnet routes advertised via Tailscale | `10.7.0.1/32`                |
| `TS_EXTRA_ARGS` | Extra arguments passed to Tailscale    | `--snat-subnet-routes=false` |

Both the `sidestore-vpn` and `tailscale` services require their own `/dev/net/tun` device mount and `NET_ADMIN` capability because Compose's `network_mode: service:sidestore-vpn` shares only the network stack, not runtime capabilities or devices. Tailscale state is persisted in `./state` so you don't need to re-authenticate on restart.

# Credit

Thanks to [SideStore](https://github.com/SideStore/SideStore) for creating an app to easily install apps on iOS devices.

Thanks to [StosVPN](https://github.com/SideStore/StosVPN) for the approach in networking.

# License

Public domain.
