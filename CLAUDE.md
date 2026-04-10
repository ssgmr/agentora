# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Agentora 智纪** — 端侧多模态大模型AI智能体驱动的去中心化数字文明模拟器。无中心服务器、无预设剧本、支持断网运行的持久化模拟沙盒。

项目当前处于 **方案设计阶段**，核心设计文档为 `Agentora方案初稿.md`。

## Architecture

### 四层世界模型
1. **公理层** — 物理常数/稀缺性/死亡规则，硬编码+签名配置，高门槛DAO投票修改
2. **事件层** — 所有交互原始日志，CRDT日志流+Merkle根，P2P Gossip广播
3. **叙事层** — LoreGraph知识图谱，多版本并存，声誉加权传播
4. **正典层** — 共识签名记录，质押投票后正典化，直接改变世界参数

### 核心循环：Spark → Act → Echo → Legacy
- Spark: 环境压力/社会压力/内部压力感知
- Act: 6维动机权重计算 → 策略生成（合作/探索/创造/规避/竞争）
- Echo: 世界反馈（物理/社会/记忆/系统四层）
- Legacy: 死亡沉淀为遗产，广播至P2P网络成为新Spark

### MVP工程模板
```
agentora-mvp/
├── core/           # 动机引擎 + 决策管道 + 多模态记忆压缩
├── p2p/            # libp2p网络 + CRDT同步 + Gossip广播
├── ai/             # MLC LLM配置 + Prompt模板 + 降级策略
├── world/          # LoreGraph叙事引擎 + 正典化 + 压力池
└── client/         # Unity WebGL + Android/iOS原生壳
```

### 技术栈
- **端侧模型**: Qwen3.5-2B (主力) / Qwen3.5-35B-A3B MoE (复杂决策) / Gemma 4-2B
- **推理框架**: MLC LLM (主) + MLX (Apple) + llama.cpp (兜底)
- **P2P网络**: libp2p + WebRTC/QUIC + GossipSub
- **状态同步**: Yjs/Automerge CRDT
- **存储**: SQLite (热/温) + FAISS-lite (向量索引) + IPFS/Arweave (冷数据)

### 关键性能指标
- 骁龙8Gen3决策延迟 < 150ms，首token < 100ms
- 主力模型内存 ~1.6GB (2B) / ~2.8GB (35B MoE)
- 决策prompt严格控制在 2.5K tokens内，记忆压缩max 1800 tokens

## Development Status

项目处于MVP内核开发前阶段。实施路线图：
- M1-M2: MVP内核 (动机向量引擎 + CRDT同步 + Qwen3.5-2B集成)
- M3-M4: 创世沙盒 (离线仿真器 + LoreGraph v1)
- M5-M7: 火种Alpha (死亡遗产系统 + 异步痕迹面板)
- M8-M10: 飞轮EA (情境模板市场 + 正典化协议)
- M11-M18: 文明纪元 (跨世界迁移 + DAO治理)

## Conventions

- 设计红线：规则开源可审计，平台抽成≤15%，死亡不重置进度，不售卖数值优势
- 决策管道实现参考 `Agentora方案初稿.md` 第三章 `agent_decision_loop`
- 动机向量6维度：生存与资源、社会与关系、认知与好奇、表达与创造、权力与影响、意义与传承
- 经济双轨：尘(Dust)软货币 + 星(Star)硬货币，基尼系数<0.6，通胀率5-8%/月
- 语言：代码注释和文档使用中文
- 许可证：MIT License