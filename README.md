# SideStore VPN tool

[SideStore](https://github.com/SideStore/SideStore) usually requires a WireGuard tunnel or [StosVPN](https://github.com/SideStore/StosVPN), so traffic generated during app install/refresh process can be hijacked and processed by SideStore itself.

This tool provides an alternative to the aforementioned tools, and can allow SideStore to work across all iOS devices on your local network, without setting up WireGuard or StosVPN individually.

# How it works

When installing or refreshing apps, SideStore opens ports on local iOS device mimicing a computer running developer software. Then it instructs iOS to connect to a computer at `10.7.0.1`.

This tool creates a TUN device expecting packets to 10.7.0.1, swap the source/destination field of each packet, and send them so that they are forwarded back to the iOS device sending the request. This will get iOS talking with SideStore's fake computer, and allow apps to be installed/refreshed.

This is the same approach as used by StosVPN: <https://github.com/SideStore/StosVPN/blob/main/TunnelProv/PacketTunnelProvider.swift>

## Docker with Tailscale

A Docker image is available at `ghcr.io/Skulldorom/sidestore-vpn`. The recommended way to run it is with Tailscale, which handles routing automatically without needing to configure static routes on your router.

```bash
docker run --rm --cap-add=NET_ADMIN -v /dev/net/tun:/dev/net/tun -v ./state:/var/lib/tailscale \
  -e TS_AUTHKEY=tskey-xxxxxxx \
  -e TS_HOSTNAME=sidestore-vpn \
  ghcr.io/Skulldorom/sidestore-vpn
```

## Docker Compose with Tailscale

Create a `.env` file with your Tailscale auth key and optionally a custom hostname:

```bash
TS_AUTHKEY=tskey-xxxxxxx
TS_HOSTNAME=sidestore-vpn  # optional, defaults to sidestore-vpn
```

Then start the service:

```bash
docker compose -f docker-compose-tailscale.yml up
```

Example `docker-compose-tailscale.yml`:

```yamlversion: "3.8"
services:
  sidestore-vpn:
    image: ghcr.io/skulldorom/sidestore-vpn:latest
    container_name: sidestore-vpn
    cap_add:
      - NET_ADMIN
    devices:
      - /dev/net/tun:/dev/net/tun
      # Uncomment the following line on hosts with a TPM module to enable hardware attestation:
      # - /dev/tpmrm0:/dev/tpmrm0
    # Uncomment the following lines to enable IP forwarding on the host (required for routing traffic through the VPN):
    # sysctls:
    #   net.ipv4.ip_forward: "1"
    #   net.ipv6.conf.all.forwarding: "1"
    environment:
      - TS_AUTHKEY=${TS_AUTHKEY}
      - TS_ROUTES=${TS_ROUTES:-10.7.0.1/32}
      - TS_EXTRA_ARGS=${TS_EXTRA_ARGS:---snat-subnet-routes=false}
      - TS_HOSTNAME=${TS_HOSTNAME:-sidestore-vpn}
    volumes:
      - ./state:/var/lib/tailscale
```

The `docker-compose-tailscale.yml` file uses the following environment variables:

| Variable        | Description                            | Default                      |
| --------------- | -------------------------------------- | ---------------------------- |
| `TS_AUTHKEY`    | Tailscale auth key (required)          | —                            |
| `TS_HOSTNAME`   | Hostname shown in Tailscale admin      | `sidestore-vpn`              |
| `TS_ROUTES`     | Subnet routes advertised via Tailscale | `10.7.0.1/32`                |
| `TS_EXTRA_ARGS` | Extra arguments passed to Tailscale    | `--snat-subnet-routes=false` |

The container requires the `/dev/net/tun` device and the `NET_ADMIN` capability to create the TUN interface. The Tailscale state is persisted in `./state` so you don't need to re-authenticate on restart.

# Credit

Thanks to [SideStore](https://github.com/SideStore/SideStore) for creating an app to easily install apps on iOS devices.

Thanks to [StosVPN](https://github.com/SideStore/StosVPN) for the approach in networking.

# License

Public domain.
