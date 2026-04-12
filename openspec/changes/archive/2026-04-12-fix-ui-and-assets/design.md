## 架构

### UI层级结构

```
Main (Node)
├── Camera2D                 # 控制世界视图
├── WorldView (Node2D)       # 地形 + Agent + 建筑
│   ├── Agents (Node2D)
│   ├── Structures (Node2D)
│   └── Legacies (Node2D)
├── TopBar (HBoxContainer)   # 屏幕顶部，横跨全屏
├── RightPanel (Control)     # 屏幕右侧，全高
└── NarrativeFeed (Control)  # 屏幕底部，左侧区域
```

**关键设计决策**：不使用 CanvasLayer。Godot 4 中 Control 节点的锚点相对 viewport rect，天然使用屏幕空间，不受 Camera2D 的 position/zoom 影响。CanvasLayer 反而引入锚点计算bug。

### UI锚点布局

```
┌──────────────────────────────────────────────┐
│ TopBar                                       │
│ anchor_top=0 left=0 right=1 offset_bottom=36  │
│ [Tick: N] [Agent: N] [速度控制 ▼]             │
├──────────────────────────┬───────────────────┤
│                          │ RightPanel        │
│ WorldView                │ anchor_left=1     │
│ (Node2D世界空间)          │ anchor_right=1    │
│ 摄像机在此区域           │ anchor_top=0      │
│ 自由平移/缩放            │ anchor_bottom=1   │
│                          │ offset_left=-320  │
│                          │                   │
│                          │ ┌───────────────┐ │
│                          │ │ AgentDetail   │ │
│                          │ │ NameLabel     │ │
│                          │ │ Radar(280×220)│ │
│                          │ │ StatusLabel   │ │
│                          │ │ GuidePanel    │ │
│                          │ └───────────────┘ │
│                          │ ┌───────────────┐ │
│                          │ │ WorldInfo     │ │
│                          │ └───────────────┘ │
├──────────────────────────┴───────────────────┤
│ NarrativeFeed                                │
│ anchor_bottom=1 left=0 right=1               │
│ offset_top=-180 offset_right=-320            │
│ ┌──────────────────────────────────────────┐ │
│ │ 事件日志（滚动显示Agent决策叙事）          │ │
│ └──────────────────────────────────────────┘ │
──────────────────────────────────────────────┘
```

### 雷达图渲染方案

当前问题：固定 `radar_size=100` 在 `custom_minimum_size=200×160` 容器中导致标签溢出。

**新方案**：
- 控件 `custom_minimum_size=280×220`（从tscn设置）
- 雷达尺寸自适应：`effective_size = min(size.x, size.y) * 0.42`
- 标签位置根据角度自动对齐（左/中/右）
- 维度名与GuidePanel统一为简称（生存/社交/认知/表达/权力/传承）
- 网格环4个（原5个），减少视觉杂乱

### SVG资源管线

```
SVG源文件 (矢量图)
    │ 批量导出
    ▼
PNG资源 (16×16 / 32×32)
    │ Godot自动导入
    ▼
Texture2D资源
    │
    ├── terrain_textures.tres (5种地形)
    ├── agent_placeholder.tres (Agent)
    ├── structure_placeholder.tres (建筑)
    └── legacy_placeholder.tres (遗迹)
```

### 地形噪声方案

当前：`sin(x*1.5) * cos(y*1.5)` 产生同心圆

**新方案**：多层分形噪声（4个八度）
- 多个角度的正弦波叠加（1.2/0.7, 1.5/-0.5, 0.8/0.8）
- 每个八度振幅减半、频率×2.1
- 归一化到[0,1]，产生自然的斑块分布

### 需要创建的SVG资源

| 资源 | 尺寸 | 用途 | 说明 |
|------|------|------|------|
| agent_idle | 32×32 | Agent默认状态 | 蓝色圆形人物 |
| agent_selected | 32×32 | Agent选中状态 | 黄色边框高亮 |
| terrain_plains | 16×16 | 草原地形 | 浅绿草地+小花 |
| terrain_forest | 16×16 | 森林地形 | 深绿+树木剪影 |
| terrain_mountain | 16×16 | 山脉地形 | 灰色三角形山峰 |
| terrain_water | 16×16 | 水域地形 | 蓝色波纹 |
| terrain_desert | 16×16 | 沙漠地形 | 黄色沙丘纹理 |
| structure_default | 16×16 | Agent建筑 | 小方块+屋顶 |
| legacy_default | 16×16 | Agent遗迹 | 灰色墓碑形 |

## 技术约束

- Godot 4.6.2
- 视口分辨率 1280×720
- 世界地图 256×256 Tile，每Tile 16px
- 摄像机初始位置 (2048, 2048)，地图中心
- 不修改Rust后端代码
- SVG导出PNG保持透明背景

## 风险

1. **SVG导出工具**：需确认有可用的SVG→PNG导出工具链。方案：用Python的cairosvg库或Inkscape命令行批量导出
2. **Godot 4锚点行为**：Control节点在Node父节点下的锚点行为可能因Godot版本有差异。方案：在main.tscn中明确设置所有4个anchor值，不依赖preset
