# Agentora Client 端运行指南

## 快速启动

### 方法 1：使用启动脚本（推荐）

```bash
# Windows (Git Bash)
bash scripts/run_client.sh

# 或直接使用批处理
scripts/run_client.bat
```

### 方法 2：手动启动

```bash
# 1. 构建 Rust GDExtension
cargo build -p agentora-bridge

# 2. 复制 DLL 到 client/bin
cp target/debug/agentora_bridge.dll client/bin/

# 3. 启动 Godot
godot --path client
```

## 预期输出

成功启动后，控制台应显示：

```
Initialize godot-rust (API v4.6.stable.official...)
Godot Engine v4.6.2.stable.official...

[SimulationBridge] 初始化模拟桥接
[AgentManager] Agent 管理器初始化
[WorldRenderer] 世界渲染器初始化
[WorldRenderer] 地图生成完成：65536 个单元格
[Main] 主场景初始化
[Main] SimulationBridge 信号已连接
[Main] 主场景就绪
[SimulationBridge] Tick 1, Agents: 5
```

## 画面内容

### 地形渲染
- **绿色** (0.3, 0.6, 0.2) - 草地 (plains)
- **深绿色** (0.1, 0.4, 0.1) - 森林 (forest)
- **灰色** (0.5, 0.5, 0.5) - 山脉 (mountain)
- **蓝色** (0.2, 0.4, 0.8) - 水域 (water)
- **黄色** (0.8, 0.7, 0.3) - 沙漠 (desert)

### Agent
- 蓝色小方块 (12x12 像素)
- 带有 Agent 名称标签
- 点击可选择（变为黄色高亮）

### UI 面板
- **顶部栏**: Tick 计数器、Agent 数量、速度控制
- **右侧面板**: Agent 详情、动机雷达图、引导控制
- **底部**: 叙事事件流

## 操作说明

### 摄像机控制
| 操作 | 效果 |
|------|------|
| 右键拖拽 | 平移地图 |
| 滚轮滚动 | 缩放视图 |
| 双击 | 聚焦 Agent |

### Agent 交互
| 操作 | 效果 |
|------|------|
| 左键点击 Agent | 选择 Agent |
| 拖动动机滑块 | 调整动机权重 |
| 点击"建议探索" | 注入探索偏好 |

### 模拟控制
| 操作 | 效果 |
|------|------|
| 速度下拉框 | 选择 1x/2x/5x/暂停 |
| 暂停按钮 | 暂停/继续模拟 |
| 重置按钮 | 恢复默认动机 |

## 技术架构

```
┌─────────────────────────────────────────────────────┐
│                    Godot Engine                      │
├─────────────────────────────────────────────────────┤
│  GDScript                                            │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │ Main.gd     │  │WorldRenderer │  │AgentManager│ │
│  └──────┬──────┘  └──────┬───────┘  └─────┬──────┘ │
│         │                │                 │        │
│         └────────────────┼─────────────────┘        │
│                          │                          │
│  ┌───────────────────────▼───────────────────────┐ │
│  │          SimulationBridge.gd                   │ │
│  │          (GDScript 包装层)                      │ │
│  └───────────────────────┬───────────────────────┘ │
├──────────────────────────┼──────────────────────────┤
│  GDExtension             │                          │
│  ┌───────────────────────▼───────────────────────┐ │
│  │     SimulationBridge (Rust)                    │ │
│  │     - Tokio 运行时                              │ │
│  │     - mpsc Channel                             │ │
│  │     - WorldSnapshot 序列化                     │ │
│  └────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

## 性能指标

- 地图大小：256 x 256 = 65,536 单元格
- 初始 Agent 数：5
- Tick 间隔：2 秒（可调整）
- 渲染帧率：60 FPS
- 内存占用：~200MB

## 常见问题

### Q: Godot 启动后黑屏/无内容
A: 检查摄像机位置，使用右键拖拽移动地图，Agent 初始在地图中心 (128, 128)

### Q: DLL 加载失败
A: 确保 `client/bin/agentora_bridge.dll` 存在，重新运行 `cargo build -p agentora-bridge`

### Q: 地图渲染慢
A: 第一帧生成地图需要时间，后续会缓存。或使用缩放功能减少可见单元格数量。

### Q: 如何调试
A: 在 Godot 编辑器中运行 (使用 `--editor` 参数)，使用内置调试器和控制台。

## 开发调试

```bash
# 以编辑器模式启动（可调试）
godot --path client --editor

# 查看详细日志
godot --path . --verbose
```
