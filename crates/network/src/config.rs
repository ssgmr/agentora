//! 网络配置结构体
//!
//! DCUtR、AutoNAT、混合穿透策略、中继 reservation 配置

use serde::{Serialize, Deserialize};
use crate::nat::ConnectionType;

/// DCUtR 配置
#[derive(Debug, Clone)]
pub struct DcutrConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 单次尝试超时时间（秒）
    pub timeout_secs: u64,
    /// 并发打洞数量
    pub concurrent_attempts: u32,
}

impl Default for DcutrConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            timeout_secs: 10,
            concurrent_attempts: 2,
        }
    }
}

/// AutoNAT 配置
#[derive(Debug, Clone)]
pub struct AutonatConfig {
    /// 是否探测内网地址
    pub only_global_ips: bool,
    /// 探测频率（秒）
    pub probe_interval_secs: u64,
    /// 探测超时（秒）
    pub probe_timeout_secs: u64,
}

impl Default for AutonatConfig {
    fn default() -> Self {
        Self {
            only_global_ips: false,
            probe_interval_secs: 30,
            probe_timeout_secs: 15,
        }
    }
}

/// 混合穿透策略配置
#[derive(Debug, Clone)]
pub struct HybridStrategyConfig {
    /// 直连超时（秒）
    pub direct_timeout_secs: u64,
    /// DCUtR 超时（秒）
    pub dcutr_timeout_secs: u64,
    /// 降级阈值：直连失败多少次后降级到 DCUtR
    pub degradation_threshold: u32,
    /// DCUtR 配置
    pub dcutr: DcutrConfig,
    /// AutoNAT 配置
    pub autonat: AutonatConfig,
    /// 是否启用 DCUtR 打洞功能，默认 true
    pub enable_dcutr: bool,
    /// 是否启用 AutoNAT NAT 探测功能，默认 true
    pub enable_autonat: bool,
}

impl Default for HybridStrategyConfig {
    fn default() -> Self {
        Self {
            direct_timeout_secs: 5,
            dcutr_timeout_secs: 15,
            degradation_threshold: 2, // 失败2次后降级
            dcutr: DcutrConfig::default(),
            autonat: AutonatConfig::default(),
            enable_dcutr: true,
            enable_autonat: true,
        }
    }
}

/// 中继 reservation 信息
#[derive(Debug, Clone, Default)]
pub struct RelayReservation {
    /// 中继节点 PeerId
    pub relay_peer_id: String,
    /// 中继地址
    pub relay_addr: String,
    /// 监听地址（电路地址）
    pub listen_addr: String,
    /// 是否激活
    pub active: bool,
}

/// 已连接节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedPeer {
    /// libp2p PeerId（完整46字符）
    pub peer_id: String,
    /// Identify 协议获取的 agent_version
    pub agent_version: String,
    /// 连接方式（Direct/Relay/Dcutr）
    pub connection_type: ConnectionType,
    /// 连接建立时间（ISO 8601 格式）
    pub connected_at: String,
    /// 是否为中继服务器
    pub is_relay_server: bool,
    /// 对方的监听地址（如果已知）
    pub listen_addr: Option<String>,
}