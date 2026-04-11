//! NAT 状态和连接类型单元测试

use agentora_network::{NatStatus, ConnectionType};
use libp2p::Multiaddr;

#[test]
fn test_nat_status_public() {
    let addr: Multiaddr = "/ip4/1.2.3.4/tcp/4001".parse().unwrap();
    let status = NatStatus::Public(addr.clone());

    assert!(status.is_public());
    assert!(!status.needs_relay());
}

#[test]
fn test_nat_status_private() {
    let status = NatStatus::Private;

    assert!(!status.is_public());
    assert!(status.needs_relay());
}

#[test]
fn test_nat_status_unknown() {
    let status = NatStatus::Unknown;

    assert!(!status.is_public());
    assert!(status.needs_relay());
}

#[test]
fn test_nat_status_default() {
    let status = NatStatus::default();

    assert_eq!(status, NatStatus::Unknown);
    assert!(!status.is_public());
    assert!(status.needs_relay());
}

#[test]
fn test_connection_type_variants() {
    let direct = ConnectionType::Direct;
    let dcutr = ConnectionType::Dcutr;
    let relay = ConnectionType::Relay;

    assert_eq!(direct, ConnectionType::Direct);
    assert_eq!(dcutr, ConnectionType::Dcutr);
    assert_eq!(relay, ConnectionType::Relay);
}

#[test]
fn test_connection_type_clone() {
    let direct = ConnectionType::Direct;
    let direct_clone = direct.clone();
    assert_eq!(direct, direct_clone);
}
