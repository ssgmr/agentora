# 需求说明书

## 背景概述

当前 setup-wizard 功能已实现核心框架（UserConfig 数据结构、Bridge API、启动检测流程），但存在严重的 UI 集成问题：

1. **settings_panel 完全没有集成**：文件存在（scenes/settings_panel.tscn、scripts/settings_panel.gd）但没有入口按钮、没有代码连接，用户无法在游戏中打开设置面板。
2. **settings_panel 功能不完整**：缺少 custom_prompt、icon 选择、p2p 配置项，仅支持 LLM 模式和 Agent 名字。
3. **UI 架构混乱**：settings_panel.gd 动态构建 UI 覆盖了 .tscn 静态节点；setup_wizard.tscn 是空壳，所有 UI 在 .gd 中动态构建，无法在编辑器预览。

本次变更旨在完善 UI 集成，修复架构问题，使用户能够在游戏中完整修改配置。

## 变更目标

- 目标1：在 main.tscn 的 TopBar 添加设置按钮入口，连接 settings_panel
- 目标2：完善 settings_panel 功能，添加 custom_prompt、icon 选择、p2p 配置项
- 目标3：修复 settings_panel.gd 与 .tscn 冲突，统一使用静态 UI 定义
- 目标4：添加 ESC 快捷键打开设置面板
- 目标5：创建共享 UI 样式库，减少 setup_wizard 与 settings_panel 的代码重复

## 功能范围

### 新增功能

| 功能标识 | 功能描述 |
| --- | --- |
| `settings-entry` | TopBar 设置按钮 + ESC 快捷键入口，打开 settings_panel |
| `shared-ui-styles` | 共享 UI 样式函数库（按钮、输入框、面板样式），供 setup_wizard 和 settings_panel 复用 |

### 修改功能

| 功能标识 | 变更说明 |
| --- | --- |
| `setup-wizard-ui` | 扩展 settings_panel 功能：添加 custom_prompt 输入、icon 选择、p2p 配置项 |
| `godot-client` | 修改 main.tscn 添加 SettingsBtn，修改 main.gd 连接 settings_panel 和快捷键 |
| `settings-panel-ui` | 修复 .gd 与 .tscn 冲突，删除动态构建，使用静态节点定义 |

## 影响范围

- **代码模块**：
  - `client/scenes/main.tscn` — 添加 SettingsBtn 节点
  - `client/scripts/main.gd` — 添加 settings_panel 连接和 ESC 快捷键处理
  - `client/scenes/settings_panel.tscn` — 扩展 UI 结构（添加缺失配置项）
  - `client/scripts/settings_panel.gd` — 删除动态构建，使用静态节点，完善配置逻辑
  - `client/scripts/shared_ui_styles.gd` — 新增共享样式函数库
  - `client/scenes/setup_wizard.tscn` — 可选：将动态 UI 迁移到静态（降低优先级）

- **API接口**：无新增，复用现有 Bridge API（get_user_config、set_user_config）

- **依赖组件**：无新增依赖

- **关联系统**：setup-wizard 变更（已完成的 UserConfig、Bridge API）

## 验收标准

- [ ] TopBar 显示设置按钮（齿轮图标或"设置"文字）
- [ ] 点击设置按钮打开 settings_panel 弹窗
- [ ] ESC 快捷键打开/关闭 settings_panel
- [ ] settings_panel 显示所有配置项：LLM 模式、Agent 名字、custom_prompt、icon 选择、p2p 模式、seed_address
- [ ] settings_panel 加载当前配置并正确显示
- [ ] settings_panel 保存配置到 user_config.toml
- [ ] 修改配置后显示"重启生效"提示
- [ ] settings_panel.gd 不再动态构建 UI，使用 .tscn 静态节点
- [ ] shared_ui_styles.gd 创建并被 setup_wizard.gd 和 settings_panel.gd 复用
- [ ] 移动端触摸友好（按钮尺寸 >= 36px）