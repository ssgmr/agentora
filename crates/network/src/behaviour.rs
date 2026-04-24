//! 网络行为定义
//!
//! AgentoraBehaviour 组合多种 libp2p 协议行为

use libp2p_gossipsub as gossipsub;
use libp2p_kad as kad;
use libp2p_relay as relay;
use libp2p_dcutr as dcutr;
use libp2p_autonat as autonat;
use libp2p_swarm_derive::NetworkBehaviour;

/// 网络行为事件枚举
pub enum AgentoraBehaviourEvent {
    Gossipsub(gossipsub::Event),
    Kademlia(kad::Event),
    RelayClient(relay::client::Event),
    Dcutr(dcutr::Event),
    Autonat(autonat::Event),
    Ping(libp2p_ping::Event),
    Identify(libp2p_identify::Event),
}

impl From<gossipsub::Event> for AgentoraBehaviourEvent {
    fn from(event: gossipsub::Event) -> Self {
        AgentoraBehaviourEvent::Gossipsub(event)
    }
}

impl From<kad::Event> for AgentoraBehaviourEvent {
    fn from(event: kad::Event) -> Self {
        AgentoraBehaviourEvent::Kademlia(event)
    }
}

impl From<relay::client::Event> for AgentoraBehaviourEvent {
    fn from(event: relay::client::Event) -> Self {
        AgentoraBehaviourEvent::RelayClient(event)
    }
}

impl From<dcutr::Event> for AgentoraBehaviourEvent {
    fn from(event: dcutr::Event) -> Self {
        AgentoraBehaviourEvent::Dcutr(event)
    }
}

impl From<autonat::Event> for AgentoraBehaviourEvent {
    fn from(event: autonat::Event) -> Self {
        AgentoraBehaviourEvent::Autonat(event)
    }
}

impl From<libp2p_ping::Event> for AgentoraBehaviourEvent {
    fn from(event: libp2p_ping::Event) -> Self {
        AgentoraBehaviourEvent::Ping(event)
    }
}

impl From<libp2p_identify::Event> for AgentoraBehaviourEvent {
    fn from(event: libp2p_identify::Event) -> Self {
        AgentoraBehaviourEvent::Identify(event)
    }
}

/// 网络行为组合
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "AgentoraBehaviourEvent")]
pub struct AgentoraBehaviour {
    #[behaviour(to_event = "AgentoraBehaviourEvent::Gossipsub")]
    pub gossipsub: gossipsub::Behaviour,
    #[behaviour(to_event = "AgentoraBehaviourEvent::Kademlia")]
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    #[behaviour(to_event = "AgentoraBehaviourEvent::RelayClient")]
    pub relay_client: relay::client::Behaviour,
    #[behaviour(to_event = "AgentoraBehaviourEvent::Dcutr")]
    pub dcutr: dcutr::Behaviour,
    #[behaviour(to_event = "AgentoraBehaviourEvent::Autonat")]
    pub autonat: autonat::Behaviour,
    #[behaviour(to_event = "AgentoraBehaviourEvent::Ping")]
    pub ping: libp2p_ping::Behaviour,
    #[behaviour(to_event = "AgentoraBehaviourEvent::Identify")]
    pub identify: libp2p_identify::Behaviour,
}