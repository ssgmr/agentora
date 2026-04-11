# 功能规格说明：P2P网络层

## ADDED Requirements

### Requirement: rust-libp2p集成

系统 SHALL 使用rust-libp2p作为P2P网络核心，支持GossipSub事件广播、KAD DHT节点发现、Circuit Relay v2 NAT穿透。

#### Scenario: 节点启动与发现

- **WHEN** 新节点启动并配置种子节点地址
- **THEN** 节点 SHALL 通过KAD DHT发现网络中的其他节点
- **AND** 在30秒内 SHALL 连接至少1个对等节点

#### Scenario: NAT穿透

- **WHEN** 节点位于NAT后无法直连
- **THEN** 系统 SHALL 通过Circuit Relay v2中继建立连接
- **AND** 同时尝试DCUtR打洞建立直连

### Requirement: GossipSub事件广播

系统 SHALL 通过GossipSub广播Agent动作事件、CRDT操作、叙事更新。每个区域对应一个GossipSub topic，节点仅订阅本地Agent所在区域及相邻区域的topic。

#### Scenario: 事件广播

- **WHEN** 本地Agent执行动作生成CRDT操作
- **THEN** 系统 SHALL 通过GossipSub广播至对应区域topic

#### Scenario: 事件接收

- **WHEN** 收到其他节点的GossipSub消息
- **THEN** 系统 SHALL 验证签名并将CRDT操作交由sync模块合并

#### Scenario: 兴趣过滤

- **WHEN** 节点Agent仅位于北区
- **THEN** 节点 SHALL 仅订阅北区及其邻区的GossipSub topic
- **AND** 不订阅远区topic，节省带宽

### Requirement: Transport抽象

系统 SHALL 定义Transport抽象层，应用层仅依赖publish/subscribe接口，不依赖具体传输实现。MVP实现rust-libp2p传输，预留WebSocket Relay降级通道。

#### Scenario: 传输层切换

- **WHEN** rust-libp2p传输不可用（编译/链接问题）
- **THEN** 系统 SHALL 可降级至WebSocket Relay传输
- **AND** 业务代码 SHALL 无需修改

### Requirement: 节点身份

系统 SHALL 为每个节点生成ed25519密钥对作为PeerId，私钥本地存储，公钥作为节点唯一标识。

#### Scenario: 首次启动生成身份

- **WHEN** 节点首次启动且无本地密钥
- **THEN** 系统 SHALL 生成新的ed25519密钥对
- **AND** 将PeerId写入配置

#### Scenario: 重启恢复身份

- **WHEN** 节点重启且本地已有密钥
- **THEN** 系统 SHALL 加载已有密钥恢复PeerId