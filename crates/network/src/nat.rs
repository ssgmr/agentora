//! NAT 状态和连接类型定义

use libp2p::Multiaddr;

/// NAT 状态（来自 AutoNAT）
#[derive(Debug, Clone, PartialEq)]
pub enum NatStatus {
    /// 公网可达，地址为观察到的公网地址
    Public(Multiaddr),
    /// 私有网络，需要中继
    Private,
    /// 未知，正在探测
    Unknown,
}

impl Default for NatStatus {
    fn default() -> Self {
        NatStatus::Unknown
    }
}

impl NatStatus {
    /// 是否为公网可达
    pub fn is_public(&self) -> bool {
        matches!(self, NatStatus::Public(_))
    }

    /// 是否需要中继
    pub fn needs_relay(&self) -> bool {
        matches!(self, NatStatus::Private | NatStatus::Unknown)
    }
}

/// 连接类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    /// 直连
    Direct,
    /// DCUtR 打洞连接
    Dcutr,
    /// 中继连接
    Relay,
}