# Agentora

<p align="center">
  <strong>Decentralized Civilization Simulator Driven by Edge AI Agents</strong>
</p>

<p align="center">
  <a href="README.md">🇨🇳 中文版本</a>
</p>

---

## 📖 Introduction

**Agentora** — A decentralized civilization simulator driven by edge multimodal LLM AI agents.

**Core Features:**
- 🚫 **No Central Server** — P2P network architecture, fully decentralized
- 📜 **No Preset Script** — Agents evolve autonomously, free from predefined rules
- 🔌 **Offline Capable** — Local persistence, continues running without network
- 🎮 **Single Agent per Player** — Players guide only, agents make autonomous decisions

---

## 🏗️ Architecture

```
agentora/
├── crates/
│   ├── core/      # Core engine: decision, world, agent, memory, strategy, storage
│   ├── ai/        # LLM layer: OpenAI/Anthropic compatible, local inference(feature gated)
│   ├── network/   # P2P network: libp2p, GossipSub, KAD DHT, NAT traversal
│   ├── sync/      # CRDT sync: LWW, G-Counter, OR-Set, signatures, Merkle
│   └── bridge/    # Godot GDExtension: SimulationBridge, WorldSnapshot serialization
├── client/        # Godot 4 client: TileMap rendering, Agent sprites, narrative feed
├── config/        # Config files: user_config.toml (user config), sim.toml, log.toml
├── worldseeds/    # World seed configurations
└── tests/         # Unit tests + integration tests
```

---

## ⚙️ Quick Start

### Requirements
- Rust 1.75+
- Godot 4.3+
- Local LLM service (LM Studio / llama.cpp etc.)

### Build & Run

```bash
# Clone repository
git clone https://github.com/your-repo/agentora.git
cd agentora

# Build all crates
cargo build

# Build GDExtension bridge library
cargo bridge

# Run unit tests
cargo test

# Run Godot client
godot --path client
```

---

## 🤖 LLM Configuration

On first launch, the setup panel guides configuration and generates `config/user_config.toml`:

```toml
[llm]
mode = "local"           # local / remote / rule_only
provider_type = "openai" # openai / anthropic (remote mode)
api_endpoint = ""        # Remote API address (remote mode)
api_token = ""           # API Token (remote mode)
model_name = ""          # Model name (remote mode)
local_model_path = ""    # Local model path (local mode)

[agent]
name = "Wanderer"        # Agent name
custom_prompt = ""       # Custom system prompt
icon_id = "default"      # Preset icon ID
custom_icon_path = ""    # Custom icon path

[p2p]
mode = "single"          # single / create / join
seed_address = ""        # Seed node address (join mode)
```

Three LLM modes supported:
- **local** — Local model inference (download GGUF models to `client/models/`)
- **remote** — OpenAI/Anthropic compatible API (LM Studio, llama.cpp, etc.)
- **rule_only** — Pure rule engine decisions (no LLM, for testing)

---

## 🌐 P2P Network

Agentora uses libp2p for decentralized communication:
- **GossipSub** — Region broadcast, nearby agent message sync
- **KAD DHT** — Node discovery and WorldSeed sync
- **NAT Traversal** — AutoNAT probe + DCUtR hole punch + Relay multi-strategy

---

## 🔄 CRDT State Sync

Conflict-free Replicated Data Types ensure multi-node data consistency:
- **LWW Register** — Last-Writer-Wins register
- **G-Counter** — Grow-only counter
- **OR-Set** — Observed-Remove Set
- **Signature Verification** — ed25519 operation signatures
- **Merkle Verification** — SHA-256 hash tree

---

## 🧠 Core Loop

**Spark → Act → Echo → Legacy**

| Stage | Description |
|-------|-------------|
| **Spark** | Environmental/social/internal pressure perception |
| **Act** | State assessment → LLM decision → Rule validation → Action execution |
| **Echo** | World feedback (physical/social/memory/system layers) |
| **Legacy** | Agent death precipitates legacy, broadcast to P2P network |

---

## 🛠️ Tech Stack

| Category | Technology |
|----------|------------|
| **LLM Integration** | OpenAI/Anthropic compatible API, llama-cpp-2(optional) |
| **Inference Service** | LM Studio, llama.cpp and other local services |
| **P2P Network** | libp2p + TCP + GossipSub + KAD DHT |
| **State Sync** | CRDT (custom implementation) |
| **Storage** | SQLite + FTS5 full-text index |
| **Rendering** | Godot 4 + GDExtension (Rust) |

---

## 📝 License

MIT License

---

<p align="center">
  Made with ❤️ by Agentora Team
</p>