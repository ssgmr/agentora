#!/bin/bash
# 打包 Windows exe：编译 bridge -> 创建 build 目录 -> 复制配置 -> Godot 导出
# 用法: bash scripts/build-windows.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_DIR"

echo "[1/4] 编译 agentora-bridge (release)..."
cargo build -p agentora-bridge --release

echo "[2/4] 创建 build 目录结构..."
rm -rf build
mkdir -p build/config
mkdir -p build/worldseeds
mkdir -p build/client/bin

# DLL 需要放在两个位置：
# 1. build/ 根目录：Godot 导出 exe 运行时从 exe 同级目录加载
# 2. build/client/bin/：Godot 导出时打包
cp -f "target/release/agentora_bridge.dll" "build/"
cp -f "target/release/agentora_bridge.dll" "build/client/bin/"
cp -rf "client/bin/"* "build/client/bin/"

echo "[3/4] 复制配置文件..."
cp -f "config/sim.toml" "build/config/"
cp -f "config/log.toml" "build/config/"
cp -f "config/llm.toml" "build/config/"
cp -f "config/user_config.toml" "build/config/" 2>/dev/null || true
cp -f "worldseeds/default.toml" "build/worldseeds/"

echo "[4/4] Godot 导出..."
cd client
godot --headless --export-release "Windows Desktop" ../build/agentora_windows.exe

echo ""
echo "[完成] build/agentora_windows.exe 已就绪"
echo "运行: cd build && agentora_windows.exe"
