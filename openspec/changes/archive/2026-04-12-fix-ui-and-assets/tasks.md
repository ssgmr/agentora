## Phase 1: 回退破坏性更改

- [x] 1.1 回退 main.tscn 的 CanvasLayer 改动，恢复 Control 节点为 Main 的直接子节点
- [x] 1.2 回退 main.gd 的节点路径改动
- [x] 1.3 删除无用的 guide_panel.tscn, narrative_feed.tscn, world_view.tscn

## Phase 2: UI布局修复

- [x] 2.1 修复 main.tscn TopBar 锚点：`anchors_preset=10`, `anchor_right=1.0`, `offset_bottom=36`
- [x] 2.2 修复 main.tscn RightPanel 锚点：`anchor_left/right=1.0`, `anchor_top=0`, `anchor_bottom=1.0`, `offset_left=-320`
- [x] 2.3 修复 main.tscn NarrativeFeed 锚点：`anchor_left=0`, `anchor_top=1.0`, `anchor_right=1.0`, `anchor_bottom=1.0`, `offset_top=-180`, `offset_right=-320`
- [x] 2.4 更新 main.gd @onready 节点路径（无CanvasLayer前缀）
- [x] 2.5 验证：启动游戏后三个面板都可见

## Phase 3: 雷达图优化

- [x] 3.1 motivation_radar.gd：雷达尺寸自适应容器（42%规则）
- [x] 3.2 motivation_radar.gd：维度标签根据角度自动对齐
- [x] 3.3 motivation_radar.gd：修复字体获取（ThemeDB.fallback_font）
- [x] 3.4 main.tscn：MotivationRadar custom_minimum_size 改为 280×220
- [x] 3.5 验证：6个维度标签不重叠

## Phase 4: 引导面板紧凑化

- [x] 4.1 guide_panel.gd：维度名称改为简称（生存/社交/认知/表达/权力/传承）
- [x] 4.2 guide_panel.gd：减小字体、间距，滑块自动拉伸
- [x] 4.3 guide_panel.gd：按钮使用 SIZE_EXPAND_FILL 均匀分布
- [x] 4.4 验证：引导面板在RightPanel内不溢出

## Phase 5: 视觉资源创建（SVG→PNG）

- [x] 5.1 创建 agent_idle.svg（32×32 蓝色圆形人物）
- [x] 5.2 创建 agent_selected.svg（32×32 黄色边框高亮）
- [x] 5.3 创建 terrain_plains.svg（16×16 草地+小花）
- [x] 5.4 创建 terrain_forest.svg（16×16 森林+树木）
- [x] 5.5 创建 terrain_mountain.svg（16×16 山峰）
- [x] 5.6 创建 terrain_water.svg（16×16 波纹）
- [x] 5.7 创建 terrain_desert.svg（16×16 沙丘）
- [x] 5.8 创建 structure_default.svg（16×16 建筑）
- [x] 5.9 创建 legacy_default.svg（16×16 遗迹）
- [x] 5.10 创建 svg_to_png.py 导出脚本（16×16和32×32两种尺寸）
- [x] 5.11 运行导出脚本生成PNG到 assets/sprites/ 和 assets/textures/
- [x] 5.12 更新 Godot .tres 资源文件引用新PNG

## Phase 6: Agent可见性增强

- [x] 6.1 agent_manager.gd：Agent尺寸从12px→24px
- [x] 6.2 agent_manager.gd：添加半透明黑色背景增强对比
- [x] 6.3 agent_manager.gd：标签添加阴影效果
- [x] 6.4 验证：Agent在地图上清晰可见

## Phase 7: 地形噪声改进

- [x] 7.1 world_renderer.gd：替换噪声函数为4层分形噪声
- [x] 7.2 simulation_bridge.gd：同步更新地图初始化噪声函数
- [x] 7.3 验证：地形呈现自然斑块分布，无同心圆

## Phase 8: 集成测试

- [x] 8.1 启动游戏，验证所有UI面板位置正确
- [x] 8.2 点击Agent，验证右侧面板显示正确
- [x] 8.3 拖拽/缩放摄像机，验证UI不跟随移动
- [x] 8.4 等待事件产生，验证底部日志正常显示
- [x] 8.5 截图记录最终效果
