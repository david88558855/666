//! Minimal STUN client (RFC 5389/8489 Binding) for the self-built P2P data
//! plane (ARCHITECTURE.md 7.8; NAT-traversal principles referenced from
//! EasyTier: STUN NAT probing, UDP hole punching incl. NAT4-NAT4, relay
//! fallback with latency-first route selection).
//!
//! Phase 4 wires this into hole punching: both peers probe their public UDP
//! mapping, the console rendezvous (`/p2p/sessions`) exchanges the results,
//! then peers attempt simultaneous-open punches; `symmetric` mappings skip
//! punching and go straight to the node relay path. Only the codec helpers
//! are unit-tested here; live probes need real STUN servers.

use anyhow::{anyhow, bail, Result};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;

const MAGIC_COOKIE: u32 = 0x2112A442;

/// Public STUN servers tried in order (host:port). Two successful probes
/// are enough for the symmetric-NAT heuristic.
pub const STUN_SERVERS: &[&str] = &[
    "stun.l.google.com:19302",
    "stun.cloudflare.com:3478",
    "stun.miwifi.com:3478",
];

/// Probed public mapping of the local UDP socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapping {
    /// Mapping observed by the first reachable STUN server.
    pub addr: SocketAddr,
    /// Ports observed by two independent servers differ -> symmetric NAT
    /// (RFC 5780-style heuristic without the OTHER-ADDRESS extension).
    /// Symmetric NATs cannot be hole-punched; use the node relay path.
    pub symmetric: bool,
}

/// RFC 5389 section 6 wire format: type(2) len(2) cookie(4) txid(12).
pub fn build_binding_request(txid: [u8; 12]) -> [u8; 20] {
    let mut msg = [0u8; 20];
    msg[0..2].copy_from_slice(&0x0001u16.to_be_bytes()); // Binding Request
    msg[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
    msg[8..20].copy_from_slice(&txid);
    msg
}

/// Extract XOR-MAPPED-ADDRESS (0x0020), falling back to MAPPED-ADDRESS
/// (0x0001), from a successful Binding Response (0x0101). Returns None on
/// any format/transaction mismatch.
pub fn parse_binding_response(rsp: &[u8], txid: &[u8; 12]) -> Option<SocketAddr> {
    if rsp.len() < 20 {
        return None;
    }
    let msg_type = u16::from_be_bytes([rsp[0], rsp[1]]);
    if msg_type != 0x0101 {
        return None;
    }
    if rsp[4..8] != MAGIC_COOKIE.to_be_bytes() || rsp[8..20] != txid[..] {
        return None;
    }
    let len = u16::from_be_bytes([rsp[2], rsp[3]]) as usize;
    let end = (20 + len).min(rsp.len());
    let mut i = 20;
    while i + 4 <= end {
        let attr = u16::from_be_bytes([rsp[i], rsp[i + 1]]);
        let alen = u16::from_be_bytes([rsp[i + 2], rsp[i + 3]]) as usize;
        let v = i + 4;
        if v + alen > end {
            return None;
        }
        if attr == 0x0020 || attr == 0x0001 {
            let body = &rsp[v..v + alen];
            if body.len() < 8 {
                return None;
            }
            let family = body[1];
            let mut port = u16::from_be_bytes([body[2], body[3]]);
            if attr == 0x0020 {
                port ^= (MAGIC_COOKIE >> 16) as u16;
            }
            match family {
                0x01 => {
                    let mut ip = [0u8; 4];
                    ip.copy_from_slice(&body[4..8]);
                    if attr == 0x0020 {
                        let x = u32::from_be_bytes(ip) ^ MAGIC_COOKIE;
                        ip.copy_from_slice(&x.to_be_bytes());
                    }
                    let ip4 = std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]);
                    return Some(SocketAddr::new(ip4.into(), port));
                }
                0x02 if body.len() >= 20 => {
                    let mut ip = [0u8; 16];
                    ip.copy_from_slice(&body[4..20]);
                    if attr == 0x0020 {
                        // X-Address = Address XOR (cookie << 96 | txid)
                        let mut mask = [0u8; 16];
                        mask[0..4].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
                        mask[4..16].copy_from_slice(txid);
                        for (b, m) in ip.iter_mut().zip(mask) {
                            *b ^= m;
                        }
                    }
                    let ip6 = std::net::Ipv6Addr::from(ip);
                    return Some(SocketAddr::new(ip6.into(), port));
                }
                _ => return None,
            }
        }
        // Attributes are 4-byte aligned per RFC 5389 section 15.
        i = v + alen.div_ceil(4) * 4;
    }
    None
}

fn rand_txid() -> [u8; 12] {
    use rand::RngCore;
    let mut t = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut t);
    t
}

/// One STUN Binding exchange against `server` from `local`.
async fn probe(server: &str, local: &UdpSocket) -> Result<SocketAddr> {
    let target = tokio::net::lookup_host(server)
        .await?
        .next()
        .ok_or_else(|| anyhow!("no address resolved for {server}"))?;
    let txid = rand_txid();
    let req = build_binding_request(txid);
    local.send_to(&req, target).await?;
    let mut buf = [0u8; 256];
    let (n, _) = tokio::time::timeout(Duration::from_secs(2), local.recv_from(&mut buf)).await??;
    parse_binding_response(&buf[..n], &txid)
        .ok_or_else(|| anyhow!("invalid binding response from {server}"))
}

/// Probe the public UDP mapping across `STUN_SERVERS` (best effort: takes
/// the first two successful answers). Fails only when no server answers.
pub async fn discover_public_mapping() -> Result<Mapping> {
    let local = UdpSocket::bind("0.0.0.0:0").await?;
    let mut results: Vec<SocketAddr> = Vec::new();
    for server in STUN_SERVERS {
        if results.len() == 2 {
            break;
        }
        if let Ok(addr) = probe(server, &local).await {
            results.push(addr);
        }
    }
    match results.as_slice() {
        [a, b] => Ok(Mapping {
            addr: *a,
            symmetric: a.port() != b.port(),
        }),
        [a] => Ok(Mapping {
            addr: *a,
            symmetric: false,
        }),
        _ => bail!("no STUN server reachable"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_request_layout() {
        let txid = [7u8; 12];
        let msg = build_binding_request(txid);
        assert_eq!(&msg[0..2], &0x0001u16.to_be_bytes()); // Binding Request
        assert_eq!(&msg[2..4], &0u16.to_be_bytes()); // no attributes
        assert_eq!(&msg[4..8], &MAGIC_COOKIE.to_be_bytes());
        assert_eq!(&msg[8..20], &txid[..]);
    }

    #[test]
    fn parses_xor_mapped_v4() {
        let txid = [0xAAu8; 12];
        let real_port = 54321u16;
        let real_ip = [203u8, 0, 113, 7];
        let xport = real_port ^ (MAGIC_COOKIE >> 16) as u16;
        let xip = u32::from_be_bytes(real_ip) ^ MAGIC_COOKIE;
        let mut rsp = vec![0u8; 32]; // 20 header + 12 attribute
        rsp[0..2].copy_from_slice(&0x0101u16.to_be_bytes()); // success response
        rsp[2..4].copy_from_slice(&12u16.to_be_bytes()); // one 12-byte attribute
        rsp[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
        rsp[8..20].copy_from_slice(&txid);
        rsp[20..22].copy_from_slice(&0x0020u16.to_be_bytes()); // XOR-MAPPED-ADDRESS
        rsp[22..24].copy_from_slice(&8u16.to_be_bytes()); // body length
        rsp[24] = 0;
        rsp[25] = 0x01; // family IPv4
        rsp[26..28].copy_from_slice(&xport.to_be_bytes());
        rsp[28..32].copy_from_slice(&xip.to_be_bytes());
        let addr = parse_binding_response(&rsp, &txid).expect("should parse");
        assert_eq!(addr.port(), real_port);
        assert_eq!(addr.ip().to_string(), "203.0.113.7");
    }

    #[test]
    fn rejects_transaction_mismatch() {
        let txid = [1u8; 12];
        let mut rsp = vec![0u8; 20];
        rsp[0..2].copy_from_slice(&0x0101u16.to_be_bytes());
        rsp[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
        rsp[8..20].copy_from_slice(&[2u8; 12]); // wrong txid
        assert!(parse_binding_response(&rsp, &txid).is_none());
    }

    #[test]
    fn parses_mapped_address_fallback() {
        let txid = [3u8; 12];
        let mut rsp = vec![0u8; 32]; // 20 header + 12 attribute
        rsp[0..2].copy_from_slice(&0x0101u16.to_be_bytes());
        rsp[2..4].copy_from_slice(&12u16.to_be_bytes());
        rsp[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
        rsp[8..20].copy_from_slice(&txid);
        rsp[20..22].copy_from_slice(&0x0001u16.to_be_bytes()); // MAPPED-ADDRESS
        rsp[22..24].copy_from_slice(&8u16.to_be_bytes()); // body length
        rsp[24] = 0;
        rsp[25] = 0x01;
        rsp[26..28].copy_from_slice(&443u16.to_be_bytes());
        rsp[28..32].copy_from_slice(&[1, 2, 3, 4]);
        let addr = parse_binding_response(&rsp, &txid).expect("should parse");
        assert_eq!(addr.to_string(), "1.2.3.4:443");
    }
}
