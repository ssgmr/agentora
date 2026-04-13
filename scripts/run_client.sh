#!/bin/bash
# Agentora Client 启动脚本
# 一键启动 Godot 客户端

set -e

# 确保 cargo 在 PATH 中
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
[[ -d "/opt/homebrew/opt/rustup/bin" ]] && export PATH="/opt/homebrew/opt/rustup/bin:$PATH"

# 项目根目录（脚本在 scripts/ 下，向上一级就是根目录）
REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CLIENT_DIR="$REPO_DIR/client"

echo "=== Agentora Client 启动脚本 ==="
echo "项目目录：$REPO_DIR"
echo "客户端目录：$CLIENT_DIR"

# 检查 Godot 是否可用
if ! command -v godot &> /dev/null; then
    echo "错误：未找到 godot 命令，请确保 Godot 已安装并添加到 PATH"
    exit 1
fi

# 检查动态库扩展名（macOS: .dylib, Linux: .so, Windows: .dll）
if [[ "$OSTYPE" == "darwin"* ]]; then
    LIB_EXT="dylib"
    LIB_PREFIX="lib"
elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    LIB_EXT="dll"
    LIB_PREFIX=""
else
    LIB_EXT="so"
    LIB_PREFIX="lib"
fi

LIB_NAME="${LIB_PREFIX}agentora_bridge.$LIB_EXT"

# 检查 GDExtension 动态库是否存在
if [ ! -f "$CLIENT_DIR/bin/$LIB_NAME" ]; then
    echo "警告：GDExtension 动态库不存在，正在构建..."
    cd "$REPO_DIR"
    cargo build -p agentora-bridge
    cp "target/debug/$LIB_NAME" "$CLIENT_DIR/bin/"
    echo "动态库已复制到 $CLIENT_DIR/bin/"
fi

echo "启动 Godot..."
cd "$CLIENT_DIR"

# 启动模式选择
if [ "$1" == "--editor" ]; then
    echo "以编辑器模式启动..."
    godot --path . --editor
elif [ "$1" == "--build" ]; then
    echo "构建 Release 版本..."
    cd "$REPO_DIR"
    cargo build --release -p agentora-bridge
    cp "target/release/$LIB_NAME" "$CLIENT_DIR/bin/"
    echo "Release 动态库已复制"
else
    echo "以运行模式启动..."
    godot --path .
fi

echo "完成"