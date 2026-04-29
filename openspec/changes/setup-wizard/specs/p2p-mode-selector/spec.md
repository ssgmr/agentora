# 功能规格说明 - P2P 模式选择

## ADDED Requirements

### Requirement: 三种世界模式

系统 SHALL 支持三种世界参与模式：单机模式、创建新世界、加入已有世界。

#### Scenario: 单机模式

- **WHEN** 用户选择单机模式
- **THEN** 系统 SHALL 跳过 P2P 网络初始化
- **AND** Simulation SHALL 以中心化模式运行
- **AND** 所有 Agent SHALL 在本地决策

#### Scenario: 创建新世界

- **WHEN** 用户选择创建新世界
- **THEN** 本节点 SHALL 作为种子节点启动
- **AND** SHALL 生成并显示本地 P2P 地址
- **AND** 地址 SHALL 供其他玩家连接

#### Scenario: 加入已有世界

- **WHEN** 用户选择加入已有世界
- **THEN** 用户 SHALL 输入种子节点地址
- **AND** 系统 SHALL 连接到种子节点
- **AND** 通过 P2P 同步世界状态

### Requirement: 创建世界地址显示

创建世界模式 SHALL 显示本地节点的 P2P 地址，支持分享给其他玩家。

#### Scenario: 地址生成显示

- **WHEN** 用户选择创建世界
- **THEN** 系统 SHALL 启动 P2P 服务
- **AND** 获取本地 PeerId 和监听地址
- **AND** 显示完整 P2P 地址（如 /ip4/192.168.1.100/tcp/4001/p2p/...）

#### Scenario: 复制地址功能

- **WHEN** 用户点击"复制地址"按钮
- **THEN** 地址 SHALL 复制到系统剪贴板
- **AND** 显示"地址已复制"提示

#### Scenario: 分享地址功能（移动端）

- **WHEN** 用户点击"分享地址"按钮
- **THEN** SHALL 调用系统分享功能
- **AND** 可分享到微信、邮件等渠道
- **AND** 分享内容 SHALL 包含完整 P2P 地址

### Requirement: 加入世界地址输入

加入世界模式 SHALL 支持输入种子节点地址，移动端支持便捷输入方式。

#### Scenario: 手动输入地址

- **WHEN** 用户输入种子节点地址
- **THEN** 输入框 SHALL 支持完整 P2P 地址格式
- **AND** 地址格式验证 SHALL 检查 /ip4/.../tcp/.../p2p/... 格式

#### Scenario: 剪贴板粘贴（移动端）

- **WHEN** 用户点击"从剪贴板粘贴"按钮
- **THEN** SHALL 从剪贴板读取地址
- **AND** 自动填入输入框

#### Scenario: 二维码扫描（可选）

- **WHEN** 用户点击"扫描二维码"按钮
- **THEN** SHALL 打开摄像头扫描界面
- **AND** 扫描成功后 SHALL 自动填入地址
- **AND** 二维码内容 SHALL 为 P2P 地址字符串

### Requirement: 连接验证

加入世界时 SHALL 验证能否成功连接种子节点。

#### Scenario: 连接成功

- **WHEN** 用户提交种子节点地址
- **THEN** 系统 SHALL 尝试连接
- **AND** 连接成功 SHALL 显示"连接成功"提示
- **AND** 切换到主场景开始游戏

#### Scenario: 连接失败处理

- **WHEN** 连接种子节点失败
- **THEN** SHALL 显示错误提示"无法连接到种子节点"
- **AND** 提示用户检查地址和网络
- **AND** 提供重试选项

### Requirement: 配置持久化

P2P 模式选择 SHALL 随 UserConfig 持久化。

#### Scenario: 配置保存格式

- **WHEN** 保存 P2P 配置到 user_config.toml
- **THEN** 格式 SHALL 为：
```toml
[p2p]
mode = "single"  # single / create / join
seed_address = ""  # join 模式时填写
```

#### Scenario: 配置加载恢复

- **WHEN** 系统启动时加载 user_config.toml
- **THEN** SHALL 根据 [p2p].mode 选择初始化方式
- **AND** join 模式 SHALL 自动连接 seed_address

### Requirement: 世界状态同步

加入世界后 SHALL 通过 P2P 同步世界状态。

#### Scenario: Agent 状态同步

- **WHEN** 加入已有世界成功
- **THEN** 系统 SHALL 通过 P2P 接收现有 Agent 状态
- **AND** 本地 SHALL 创建 ShadowAgent 作为远程 Agent 的代理
- **AND** 本地 Agent SHALL 通过 P2P 广播其动作

#### Scenario: 地形一致性

- **WHEN** 加入世界
- **THEN** 本地 SHALL 使用相同的 random_seed 生成地形
- **AND** 地形 SHALL 与种子节点一致
- **AND** 无需同步完整地形数据（确定性生成）