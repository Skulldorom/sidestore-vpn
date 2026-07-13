use clap::Parser;
use smoltcp::wire::{Ipv4Address, Ipv4Packet};
use std::io::{Read, Write};
use tun::AbstractDevice;

const SIDESTORE_INTERFACE_ADDR: Ipv4Address = Ipv4Address::new(10, 7, 0, 0);
const SIDESTORE_DESTINATION_ADDR: Ipv4Address = Ipv4Address::new(10, 7, 0, 1);

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

    println!("sidestore-vpn healthcheck ok");
    Ok(())
}

fn healthcheck_packet() -> [u8; 20] {
    let mut packet_buf = [0u8; 20];
    let mut packet = Ipv4Packet::new_unchecked(&mut packet_buf[..]);
    packet.set_version(4);
    packet.set_header_len(5);
    packet.set_total_len(20);
    packet.set_src_addr(SIDESTORE_INTERFACE_ADDR);
    packet.set_dst_addr(SIDESTORE_DESTINATION_ADDR);
    packet_buf
}

fn rewrite_sidestore_packet(packet_buf: &mut [u8]) -> bool {
    let Ok(mut ipv4_packet) = Ipv4Packet::new_checked(packet_buf) else {
        return false;
    };

    let dst_addr = ipv4_packet.dst_addr();
    if dst_addr != SIDESTORE_DESTINATION_ADDR {
        return false;
    }

    let src_addr = ipv4_packet.src_addr();
    ipv4_packet.set_dst_addr(src_addr);
    ipv4_packet.set_src_addr(dst_addr);
    true
}

#[cfg(test)]
mod tests {
    use super::{SIDESTORE_DESTINATION_ADDR, healthcheck_packet, rewrite_sidestore_packet};
    use smoltcp::wire::{Ipv4Address, Ipv4Packet};

    #[test]
    fn rewrites_packets_sent_to_sidestore_destination() {
        let src_addr = Ipv4Address::new(100, 64, 0, 2);
        let mut packet_buf = ipv4_packet(src_addr, SIDESTORE_DESTINATION_ADDR);

        assert!(rewrite_sidestore_packet(&mut packet_buf));

        let rewritten_packet = Ipv4Packet::new_checked(&packet_buf[..]).unwrap();
        assert_eq!(rewritten_packet.src_addr(), SIDESTORE_DESTINATION_ADDR);
        assert_eq!(rewritten_packet.dst_addr(), src_addr);
    }

    #[test]
    fn ignores_packets_for_other_destinations() {
        let src_addr = Ipv4Address::new(100, 64, 0, 2);
        let dst_addr = Ipv4Address::new(8, 8, 8, 8);
        let mut packet_buf = ipv4_packet(src_addr, dst_addr);

        assert!(!rewrite_sidestore_packet(&mut packet_buf));

        let packet = Ipv4Packet::new_checked(&packet_buf[..]).unwrap();
        assert_eq!(packet.src_addr(), src_addr);
        assert_eq!(packet.dst_addr(), dst_addr);
    }

    #[test]
    fn healthcheck_packet_targets_sidestore_destination() {
        let packet_buf = healthcheck_packet();

        let packet = Ipv4Packet::new_checked(&packet_buf[..]).unwrap();
        assert_eq!(packet.dst_addr(), SIDESTORE_DESTINATION_ADDR);
    }

    fn ipv4_packet(src_addr: Ipv4Address, dst_addr: Ipv4Address) -> [u8; 20] {
        let mut packet_buf = [0u8; 20];
        let mut packet = Ipv4Packet::new_unchecked(&mut packet_buf[..]);
        packet.set_version(4);
        packet.set_header_len(5);
        packet.set_total_len(20);
        packet.set_src_addr(src_addr);
        packet.set_dst_addr(dst_addr);
        packet_buf
    }
}
