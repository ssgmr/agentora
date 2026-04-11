# Agentora 桌面打包配置

## Windows打包
1. 安装Godot 4.6
2. 配置导出模板: 编辑器 → 编辑器设置 → 导出 → Windows
3. 编译Rust GDExtension: `cargo build --release`
4. 导出项目: 项目 → 导出 → Windows Desktop

## macOS打包
1. 安装Godot 4.6
2. 配置导出模板: 编辑器 → 编辑器设置 → 导出 → macOS
3. 编译Rust GDExtension: `cargo build --release --target aarch64-apple-darwin`
4. 导出项目: 项目 → 导出 → macOS

## Linux打包
1. 安装Godot 4.6
2. 配置导出模板: 编辑器 → 编辑器设置 → 导出 → Linux
3. 编译Rust GDExtension: `cargo build --release`
4. 导出项目: 项目 → 导出 → Linux/X11

## 打包命令

```bash
# Windows
godot --path client --export-release "Windows Desktop" agentora_windows.exe

# macOS
godot --path client --export-release "macOS" agentora_macos.dmg

# Linux
godot --path client --export-release "Linux/X11" agentora_linux.AppImage
```

## 分发内容
- 可执行文件
- WorldSeed.toml (默认配置)
- README.txt (使用说明)