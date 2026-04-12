## ADDED Requirements

### Requirement: UI控件固定在屏幕空间
系统 SHALL 确保所有UI控件（TopBar、RightPanel、NarrativeFeed）固定在屏幕边缘，不受摄像机平移/缩放影响。

#### Scenario: 摄像机移动时UI保持固定
- **WHEN** 用户拖拽移动摄像机
- **THEN** TopBar、RightPanel、NarrativeFeed保持在屏幕固定位置不移动

#### Scenario: 摄像机缩放时UI保持固定
- **WHEN** 用户使用滚轮缩放摄像机
- **THEN** TopBar、RightPanel、NarrativeFeed保持在屏幕固定位置不缩放

### Requirement: 顶部状态栏横跨屏幕顶部
系统 SHALL 在屏幕顶部显示状态栏，包含Tick计数、Agent数量、速度控制。

#### Scenario: 状态栏横跨全屏
- **GIVEN** 游戏已启动
- **THEN** TopBar横跨屏幕全宽，高度36px，位于屏幕最顶部

#### Scenario: 状态栏显示正确信息
- **GIVEN** 模拟运行中
- **THEN** TopBar显示当前Tick数、Agent数量，速度控制下拉框可切换1x/2x/5x/暂停

### Requirement: 右侧面板固定在屏幕右侧
系统 SHALL 在屏幕右侧显示320px宽的全高面板，包含Agent详情、世界信息。

#### Scenario: 右侧面板全高显示
- **GIVEN** 游戏已启动
- **THEN** RightPanel固定在屏幕右侧320px宽，从顶部延伸到底部

#### Scenario: 选中Agent显示详情
- **WHEN** 用户点击一个Agent
- **THEN** RightPanel/AgentDetail显示Agent名称、雷达图、状态信息、动机滑块

### Requirement: 事件日志固定在屏幕底部
系统 SHALL 在屏幕底部左侧显示事件日志面板，高度180px。

#### Scenario: 事件日志可见
- **GIVEN** 游戏已启动
- **THEN** NarrativeFeed在屏幕底部可见，右侧留320px给RightPanel，高度180px

#### Scenario: 事件日志自动滚动
- **WHEN** 新事件产生
- **THEN** 日志自动滚动到最新事件，最多保留100条

## MODIFIED Requirements

### Requirement: 雷达图标签不重叠
系统 SHALL 确保6维雷达图的维度标签清晰可辨，不互相重叠。

#### Scenario: 雷达图标签自适应对齐
- **GIVEN** 右侧面板尺寸≥280×220
- **WHEN** 雷达图渲染
- **THEN** 6个维度标签根据角度自动采用左/中/右对齐，不互相重叠

#### Scenario: 雷达图尺寸自适应容器
- **GIVEN** 控件容器尺寸变化
- **WHEN** 雷达图渲染
- **THEN** 雷达尺寸按容器最小维度的42%自适应缩放

### Requirement: 引导面板紧凑布局
系统 SHALL 提供紧凑的动机调整面板，维度名称使用简称。

#### Scenario: 维度名称显示简称
- **GIVEN** 引导面板渲染
- **THEN** 6个维度名称使用简称：生存/社交/认知/表达/权力/传承

#### Scenario: 滑块和按钮紧凑排列
- **GIVEN** 引导面板在RightPanel内
- **THEN** 滑块自动拉伸填充可用宽度，按钮均匀分布
