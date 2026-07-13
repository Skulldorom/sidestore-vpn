use clap::Parser;
use smoltcp::wire::{Ipv4Address, Ipv4Packet};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::time::Duration;
use tun::AbstractDevice;

const SIDESTORE_INTERFACE_ADDR: Ipv4Address = Ipv4Address::new(10, 7, 0, 0);
const SIDESTORE_DESTINATION_ADDR: Ipv4Address = Ipv4Address::new(10, 7, 0, 1);
const HEALTHCHECK_TIMEOUT: Duration = Duration::from_secs(2);
const HEALTHCHECK_PAYLOAD: &[u8] = b"sidestore-vpn-healthcheck";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the TUN interface
    #[arg(short, long, default_value = "sidestore")]
    tun_name: String,

    /// Run a lightweight self-check suitable for the scratch container image
    #[arg(long)]
    healthcheck: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.healthcheck {
        run_healthcheck()?;
        return Ok(());
    }

    // Set up Ctrl+C handler to exit immediately
    ctrlc::set_handler(|| {
        std::process::exit(0);
    })?;

    let mut config = tun::Configuration::default();
    config.tun_name(&args.tun_name);
    config.up();

    let mut dev = tun::create(&config)?;
    dev.set_address(std::net::IpAddr::V4(SIDESTORE_INTERFACE_ADDR))
        .expect("Failed to set interface address");
    dev.set_destination(std::net::IpAddr::V4(SIDESTORE_DESTINATION_ADDR))
        .expect("Failed to set destination address");
    dev.enabled(true).expect("Failed to enable interface");

    println!("TUN device \"{}\" is up", args.tun_name);

    let mut buf = [0u8; 1504]; // MTU of 1500 + 4 bytes for header

    loop {
        let n = dev.read(&mut buf)?;
        let packet_buf = &mut buf[..n];

        if rewrite_sidestore_packet(packet_buf) {
            dev.write_all(packet_buf)?;
        }
    }
}

fn run_healthcheck() -> Result<(), Box<dyn std::error::Error>> {
    let mut packet_buf = healthcheck_packet();
    if !rewrite_sidestore_packet(&mut packet_buf) {
        return Err("failed to rewrite healthcheck packet".into());
    }

    let rewritten_packet = Ipv4Packet::new_checked(&packet_buf[..])?;
    if rewritten_packet.src_addr() != SIDESTORE_DESTINATION_ADDR
        || rewritten_packet.dst_addr() != SIDESTORE_INTERFACE_ADDR
    {
        return Err("healthcheck packet was rewritten incorrectly".into());
    }

    check_sidestore_destination_reachable()?;

    println!("sidestore-vpn healthcheck ok");
    Ok(())
}

fn check_sidestore_destination_reachable() -> Result<(), Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))?;
    let local_port = socket.local_addr()?.port();
    let destination = SocketAddrV4::new(SIDESTORE_DESTINATION_ADDR, local_port);

    socket.set_read_timeout(Some(HEALTHCHECK_TIMEOUT))?;
    socket.set_write_timeout(Some(HEALTHCHECK_TIMEOUT))?;
    socket.connect(destination)?;
    socket.send(HEALTHCHECK_PAYLOAD)?;

    let mut response = [0u8; 64];
    let response_len = socket.recv(&mut response)?;
    if &response[..response_len] != HEALTHCHECK_PAYLOAD {
        return Err("healthcheck probe received unexpected payload".into());
    }

    Ok(())
}

fn healthcheck_packet() -> [u8; 20] {
    let mut packet_buf = [0u8; 20];
    let mut packet = Ipv4Packet::new_unchecked(&mut packet_buf[..]);
    packet.set_version(4);
    packet.set_header_len(20);
    packet.set_total_len(20);
    packet.set_src_addr(SIDESTORE_INTERFACE_ADDR);
    packet.set_dst_addr(SIDESTORE_DESTINATION_ADDR);
    packet.fill_checksum();
    packet_buf
}

fn rewrite_sidestore_packet(packet_buf: &mut [u8]) -> bool {
    let Ok(mut ipv4_packet) = Ipv4Packet::new_checked(packet_buf) else {
        return false;
    };

    if ipv4_packet.version() != 4 {
        return false;
    }

    let dst_addr = ipv4_packet.dst_addr();
    if dst_addr != SIDESTORE_DESTINATION_ADDR {
        return false;
    }

    let src_addr = ipv4_packet.src_addr();
    ipv4_packet.set_dst_addr(src_addr);
    ipv4_packet.set_src_addr(dst_addr);
    ipv4_packet.fill_checksum();
    true
}

#[cfg(test)]
mod tests {
    use super::{
        SIDESTORE_DESTINATION_ADDR, SIDESTORE_INTERFACE_ADDR, healthcheck_packet,
        rewrite_sidestore_packet,
    };
    use smoltcp::wire::{IpProtocol, Ipv4Address, Ipv4Packet};

    const TEST_SRC_ADDR: Ipv4Address = Ipv4Address::new(100, 64, 0, 2);
    const OTHER_DST_ADDR: Ipv4Address = Ipv4Address::new(8, 8, 8, 8);
    const TEST_PAYLOAD: &[u8] = b"sidestore-test-payload";

    #[test]
    fn rewrites_packets_sent_to_sidestore_destination() {
        let mut packet_buf = ipv4_packet(TEST_SRC_ADDR, SIDESTORE_DESTINATION_ADDR, TEST_PAYLOAD);

        assert!(rewrite_sidestore_packet(&mut packet_buf));

        let rewritten_packet = Ipv4Packet::new_checked(&packet_buf[..]).unwrap();
        assert_eq!(rewritten_packet.src_addr(), SIDESTORE_DESTINATION_ADDR);
        assert_eq!(rewritten_packet.dst_addr(), TEST_SRC_ADDR);
        assert_eq!(rewritten_packet.payload(), TEST_PAYLOAD);
        assert!(rewritten_packet.verify_checksum());
    }

    #[test]
    fn preserves_non_address_ipv4_header_fields_when_rewriting() {
        let mut packet_buf = ipv4_packet(TEST_SRC_ADDR, SIDESTORE_DESTINATION_ADDR, TEST_PAYLOAD);
        let original = Ipv4Packet::new_checked(&packet_buf[..]).unwrap();
        let original_header_len = original.header_len();
        let original_total_len = original.total_len();
        let original_ident = original.ident();
        let original_hop_limit = original.hop_limit();
        let original_next_header = original.next_header();
        let original_dont_frag = original.dont_frag();

        assert!(rewrite_sidestore_packet(&mut packet_buf));

        let rewritten = Ipv4Packet::new_checked(&packet_buf[..]).unwrap();
        assert_eq!(rewritten.header_len(), original_header_len);
        assert_eq!(rewritten.total_len(), original_total_len);
        assert_eq!(rewritten.ident(), original_ident);
        assert_eq!(rewritten.hop_limit(), original_hop_limit);
        assert_eq!(rewritten.next_header(), original_next_header);
        assert_eq!(rewritten.dont_frag(), original_dont_frag);
    }

    #[test]
    fn ignores_packets_for_other_destinations() {
        let mut packet_buf = ipv4_packet(TEST_SRC_ADDR, OTHER_DST_ADDR, TEST_PAYLOAD);
        let original_packet_buf = packet_buf.clone();

        assert!(!rewrite_sidestore_packet(&mut packet_buf));
        assert_eq!(packet_buf, original_packet_buf);

        let packet = Ipv4Packet::new_checked(&packet_buf[..]).unwrap();
        assert_eq!(packet.src_addr(), TEST_SRC_ADDR);
        assert_eq!(packet.dst_addr(), OTHER_DST_ADDR);
        assert!(packet.verify_checksum());
    }

    #[test]
    fn rejects_truncated_packets_without_mutating_them() {
        let mut packet_buf = [0xabu8; 12];
        let original_packet_buf = packet_buf;

        assert!(!rewrite_sidestore_packet(&mut packet_buf));
        assert_eq!(packet_buf, original_packet_buf);
    }

    #[test]
    fn rejects_packets_with_invalid_ipv4_version_without_mutating_them() {
        let mut packet_buf = ipv4_packet(TEST_SRC_ADDR, SIDESTORE_DESTINATION_ADDR, TEST_PAYLOAD);
        packet_buf[0] = (6 << 4) | 5;
        let original_packet_buf = packet_buf.clone();

        assert!(!rewrite_sidestore_packet(&mut packet_buf));
        assert_eq!(packet_buf, original_packet_buf);
    }

    #[test]
    fn rejects_packets_with_inconsistent_total_length_without_mutating_them() {
        let mut packet_buf = ipv4_packet(TEST_SRC_ADDR, SIDESTORE_DESTINATION_ADDR, TEST_PAYLOAD);
        packet_buf[2] = 0xff;
        packet_buf[3] = 0xff;
        let original_packet_buf = packet_buf.clone();

        assert!(!rewrite_sidestore_packet(&mut packet_buf));
        assert_eq!(packet_buf, original_packet_buf);
    }

    #[test]
    fn healthcheck_packet_is_a_valid_minimal_ipv4_packet() {
        let packet_buf = healthcheck_packet();

        let packet = Ipv4Packet::new_checked(&packet_buf[..]).unwrap();
        assert_eq!(packet.version(), 4);
        assert_eq!(packet.header_len(), 20);
        assert_eq!(packet.total_len(), 20);
        assert_eq!(packet.src_addr(), SIDESTORE_INTERFACE_ADDR);
        assert_eq!(packet.dst_addr(), SIDESTORE_DESTINATION_ADDR);
        assert!(packet.verify_checksum());
    }

    fn ipv4_packet(src_addr: Ipv4Address, dst_addr: Ipv4Address, payload: &[u8]) -> Vec<u8> {
        let packet_len = 20 + payload.len();
        let mut packet_buf = vec![0u8; packet_len];
        let mut packet = Ipv4Packet::new_unchecked(&mut packet_buf[..]);
        packet.set_version(4);
        packet.set_header_len(20);
        packet.set_total_len(packet_len as u16);
        packet.set_ident(0x1234);
        packet.set_dont_frag(true);
        packet.set_hop_limit(64);
        packet.set_next_header(IpProtocol::Udp);
        packet.set_src_addr(src_addr);
        packet.set_dst_addr(dst_addr);
        packet.payload_mut().copy_from_slice(payload);
        packet.fill_checksum();
        packet_buf
    }
}
