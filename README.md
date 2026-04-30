# Agentora 智纪

<p align="center">
  <strong>端侧智能体驱动的去中心化文明模拟器</strong>
</p>

<p align="center">
  <a href="README_EN.md">🇺🇸 English Version</a>
</p>

---

## 📖 项目简介

**Agentora 智纪** — 端侧多模态大模型AI智能体驱动的去中心化数文明模拟器。

**核心特性：**
- 🚫 **无中心服务器** — P2P网络架构，完全去中心化
- 📜 **无预设剧本** — Agent自主演化，不受预设规则限制
- 🔌 **支持断网运行** — 本地持久化，离线状态下可持续运行
- 🎮 **单Agent制** — 每个玩家只有一个Agent，玩家仅做引导，Agent自主决策

---

## 🏗️ 项目架构

```
agentora/
├── crates/
│   ├── core/      # 核心引擎：决策、世界、Agent、记忆、策略、存储
│   ├── ai/        # LLM接入层：OpenAI/Anthropic兼容端点、本地推理(feature门控)
│   ├── network/   # P2P网络：libp2p、GossipSub、KAD DHT、NAT穿透
│   ├── sync/      # CRDT同步：LWW、G-Counter、OR-Set、签名、Merkle
│   └── bridge/    # Godot GDExtension：SimulationBridge、WorldSnapshot序列化
├── client/        # Godot 4客户端：TileMap渲染、Agent Sprite、叙事流面板
├── config/        # 配置文件：llm.toml, sim.toml, log.toml
├── worldseeds/    # 世界种子配置
└── tests/         # 单元测试 + 集成测试
```

---

## ⚙️ 快速开始

### 环境要求
- Rust 1.75+
- Godot 4.3+
- 本地LLM服务（LM Studio / llama.cpp 等）

### 构建与运行

```bash
# 克隆项目
git clone https://github.com/your-repo/agentora.git
cd agentora

# 构建所有crates
cargo build

# 构建GDExtension桥接库
cargo bridge

# 运行单元测试
cargo test

# 运行Godot客户端
godot --path client
```

---

## 🤖 LLM 配置

编辑 `config/llm.toml`：

```toml
[primary]
provider = "openai"
api_base = "http://localhost:1234"
model = "qwen3.5-4b@q4_k_m"

[anthropic_compat]
provider = "anthropic"
api_base = "http://localhost:1234"
model = "gemma-4-e2b-it"

[decision]
max_tokens = 1024
temperature = 0.7
prompt_max_tokens = 6000
```

支持两种接入方式：
- **OpenAI Compatible** — LM Studio、本地OpenAI兼容API服务
- **Anthropic Compatible** — Anthropic兼容端点（备用）
- **本地推理** — llama-cpp-2（需启用 `local-inference` feature）

---

## 🌐 P2P 网络

Agentora 使用 libp2p 实现去中心化通信：
- **GossipSub** — 区域广播，邻近Agent消息同步
- **KAD DHT** — 节点发现与WorldSeed同步
- **NAT穿透** — AutoNAT探测 + DCUtR打洞 + Relay中继多策略

---

## 🔄 CRDT 状态同步

无冲突复制数据类型确保多节点数据一致性：
- **LWW Register** — Last-Writer-Wins注册表
- **G-Counter** — 增长计数器
- **OR-Set** — Observed-Remove Set
- **签名验证** — ed25519操作签名
- **Merkle校验** — SHA-256哈希树

---

## 🧠 核心循环

**Spark → Act → Echo → Legacy**

| 阶段 | 描述 |
|------|------|
| **Spark** | 环境压力/社会压力/内部压力感知 |
| **Act** | 状态评估 → LLM决策 → 规则校验 → 动作执行 |
| **Echo** | 世界反馈（物理/社会/记忆/系统四层） |
| **Legacy** | Agent死亡沉淀为遗产，广播至P2P网络 |

---

## 🛠️ 技术栈

| 类别 | 技术 |
|------|------|
| **LLM接入** | OpenAI/Anthropic兼容API、llama-cpp-2(可选) |
| **推理服务** | LM Studio、llama.cpp等本地服务 |
| **P2P网络** | libp2p + TCP + GossipSub + KAD DHT |
| **状态同步** | CRDT (自定义实现) |
| **存储** | SQLite + FTS5 全文索引 |
| **渲染** | Godot 4 + GDExtension (Rust) |

---

## 📝 许可证

MIT License

---

<p align="center">
  Made with ❤️ by Agentora Team
</p>