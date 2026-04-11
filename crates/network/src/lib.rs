//! Agentora P2P 网络层
//!
//! rust-libp2p 集成，GossipSub 广播、KAD DHT 发现、Circuit Relay 穿透。

pub mod transport;
pub mod libp2p_transport;
pub mod gossip;
pub mod codec;

pub use transport::Transport;
pub use codec::{CrdtOp, NetworkMessage};
pub use libp2p_transport::{
    Libp2pTransport, NatStatus, ConnectionType,
    DcutrConfig, AutonatConfig, HybridStrategyConfig,
};
