//! Agentora P2P 网络层
//!
//! rust-libp2p 集成，GossipSub 广播、KAD DHT 发现、Circuit Relay 穿透。

pub mod transport;
pub mod libp2p_transport;
pub mod gossip;
pub mod codec;
pub mod behaviour;
pub mod nat;
pub mod config;
pub mod swarm;

pub use transport::Transport;
pub use codec::{CrdtOp, NetworkMessage, AgentDeltaMessage, NarrativeMessage};
pub use gossip::NullMessageHandler;
pub use libp2p_transport::Libp2pTransport;
pub use swarm::SwarmCommand;
pub use behaviour::{AgentoraBehaviour, AgentoraBehaviourEvent};
pub use nat::{NatStatus, ConnectionType};
pub use config::{DcutrConfig, AutonatConfig, HybridStrategyConfig, RelayReservation, ConnectedPeer};
