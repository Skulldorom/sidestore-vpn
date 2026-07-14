# SideStore VPN

Run one SideStore VPN endpoint for every iOS device on your local network.

SideStore normally needs a per-device WireGuard tunnel or StosVPN profile so install and refresh traffic can be redirected back into the SideStore app. This project does the same packet rewrite on your network instead: iOS connects to `10.7.0.1`, `sidestore-vpn` swaps the packet source and destination addresses, and the traffic is sent back to the device running SideStore.

The recommended deployment is Docker Compose with Tailscale. Tailscale handles remote routing into your network; `sidestore-vpn` handles the SideStore packet rewrite. Small team, grim job, no tiny VPN profiles to babysit.

## How it works

When SideStore installs or refreshes apps, it opens local ports on the iOS device that mimic a computer running developer software. iOS is then told to connect to `10.7.0.1`.

`sidestore-vpn` creates a TUN interface, watches for packets addressed to `10.7.0.1`, swaps the IPv4 source and destination addresses, fixes the checksum, and writes the packet back out. That sends the connection back to the iOS device, where SideStore is listening.

This follows the same networking idea used by StosVPN: <https://github.com/SideStore/StosVPN/blob/main/TunnelProv/PacketTunnelProvider.swift>

## Requirements

- Docker with Docker Compose
- Linux host with `/dev/net/tun`
- Permission to run containers with `NET_ADMIN`
- A Tailscale account
- A reusable Tailscale auth key

## Quick start

1. Create a reusable Tailscale auth key.

   In the Tailscale admin console, create an auth key for the tailnet where your iOS devices will connect. Reusable is recommended because the container may need to re-authenticate if you remove its local state.

2. Create a `.env` file next to `docker-compose.yml`:

   ```bash
   TS_AUTHKEY=tskey-auth-your-key-here
   TS_HOSTNAME=sidestore-vpn
   ```

3. Start the stack:

   ```bash
   docker compose up -d
   ```

4. Approve the subnet route in Tailscale.

   The Compose file advertises `10.7.0.1/32` by default. Open the Tailscale admin console, find the `sidestore-vpn` machine, and approve the advertised route if Tailscale has not already accepted it.

5. Point SideStore/iOS traffic at the tailnet path and refresh/install as usual.

## Verify it is working

Check both containers are running:

```bash
docker compose ps
```

Check the VPN service logs:

```bash
docker logs sidestore-vpn
```

You should see the TUN device come up.

Check the Tailscale sidecar logs:

```bash
docker logs sidestore-vpn-tailscale
```

Then confirm in the Tailscale admin console that:

- the `sidestore-vpn` machine is online
- it advertises `10.7.0.1/32`
- that route is approved/enabled

The `sidestore-vpn` container also has a Docker healthcheck that runs:

```bash
/sidestore-vpn --healthcheck
```

## Configuration

The Compose file runs two containers:

- `sidestore-vpn` — the Rust packet-rewrite service
- `tailscale` — the official `tailscale/tailscale:stable` sidecar

Both containers need their own `/dev/net/tun` mount and `NET_ADMIN` capability. Docker Compose's `network_mode: service:sidestore-vpn` shares the network namespace, but it does not share runtime capabilities or device mounts. Yes, Docker is very literal. No, it will not take a hint.

### Environment variables

| Variable | Default | Purpose |
| --- | --- | --- |
| `TS_AUTHKEY` | required | Tailscale auth key used to join your tailnet. |
| `TS_HOSTNAME` | `sidestore-vpn` | Hostname shown in the Tailscale admin console. |
| `TS_ROUTES` | `10.7.0.1/32` | Subnet route advertised to your tailnet. |
| `TS_EXTRA_ARGS` | `--snat-subnet-routes=false` | Extra flags passed to `tailscale up`. |
| `TS_USERSPACE` | `false` | Forces kernel networking through `/dev/net/tun`. Required for this setup. |
| `TS_STATE_DIR` | `/var/lib/tailscale` | Directory where Tailscale stores node identity and state. |
| `TS_AUTH_ONCE` | `true` | Skips re-authentication when persisted Tailscale state already exists. |

### Why `TS_USERSPACE=false` matters

The official Tailscale container defaults to userspace networking. That mode terminates connections inside Tailscale's netstack instead of forwarding packets through the kernel TUN device.

This project needs real L3 packets to enter the shared network namespace so `sidestore-vpn` can rewrite packets addressed to `10.7.0.1`. `TS_USERSPACE=false` forces Tailscale to use kernel networking via `/dev/net/tun`, which is why the Tailscale sidecar also has `NET_ADMIN` and the TUN device mounted.

### Persisted Tailscale state

The Compose file mounts:

```yaml
./state:/var/lib/tailscale
```

`TS_STATE_DIR=/var/lib/tailscale` tells Tailscale to store its state in that mounted directory, and `TS_AUTH_ONCE=true` tells it not to re-authenticate when that state already exists. Together, those settings keep the container from creating a fresh Tailscale node every time it restarts.

If you want to intentionally re-enroll the node, stop the stack and remove `./state` before starting it again.

## Updating

Pull newer images and restart:

```bash
docker compose pull
docker compose up -d
```

The Rust binary image and the Tailscale image are separate. Updating one does not force the other to be rebuilt or re-downloaded.

## Troubleshooting

### The Tailscale node appears, but SideStore traffic does not work

Check that the `10.7.0.1/32` route is approved in the Tailscale admin console. Advertising a route and approving a route are annoyingly different chores.

### Tailscale creates a new machine on every restart

Make sure the `./state:/var/lib/tailscale` volume exists and that `TS_STATE_DIR=/var/lib/tailscale` is set. Do not delete `./state` unless you want to re-enroll the node.

### The containers fail to start with TUN or permission errors

Confirm the host has `/dev/net/tun`:

```bash
ls -l /dev/net/tun
```

Also confirm your Docker environment allows `NET_ADMIN` and device mounts.

### The VPN container is unhealthy

Check the logs first:

```bash
docker logs sidestore-vpn
```

The healthcheck exercises the packet rewrite path and sends a UDP probe to `10.7.0.1`. If the route is not active yet, the healthcheck can fail even though the binary itself started correctly.

## Credit

Thanks to [SideStore](https://github.com/SideStore/SideStore) for making app install and refresh on iOS less painful.

Thanks to [StosVPN](https://github.com/SideStore/StosVPN) for the original packet-rewrite networking approach.

## License

Public domain.
