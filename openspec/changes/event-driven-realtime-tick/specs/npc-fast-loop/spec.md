# 功能规格说明

## ADDED Requirements

### Requirement: 纹理资源重新生成

所有 PNG 纹理文件必须从 SVG 源文件重新生成，确保资源格式正确、Godot 可正常导入。

#### Scenario: SVG 转 PNG 完整导出
- **WHEN** 执行 `svg_to_png.py` 脚本
- **THEN** 所有 SVG 源文件被转换为对应尺寸和路径的 PNG 文件
- **AND** Sprite（32x32）：agent_idle.png、agent_selected.png
- **AND** Texture（16x16）：terrain_plains.png、terrain_forest.png、terrain_mountain.png、terrain_water.png、terrain_desert.png、structure_default.png、legacy_default.png

#### Scenario: agent.svg 纳入导出
- **WHEN** `agent.svg` 存在于 sprites 目录
- **THEN** 将其导出为 agent.png（32x32）
- **AND** 脚本不报错跳过

### Requirement: Godot 导入缓存修复

Godot 的 `.godot/imported/` 缓存必须被清理并重新导入，确保所有纹理资源可正常加载。

#### Scenario: 缓存清理后重新导入
- **WHEN** 删除 `.godot/imported/` 目录
- **AND** Godot 编辑器启动
- **THEN** Godot 自动重新导入所有 PNG 纹理
- **AND** 启动日志无纹理加载错误

#### Scenario: 纹理正确显示
- **WHEN** 世界渲染器加载地形纹理
- **AND** Agent 管理器加载 Agent 纹理
- **THEN** 纹理加载成功，不触发颜色回退
- **AND** Godot 控制台无 ERROR 级别的资源加载失败日志
