#!/bin/bash
# Agentora Client 启动脚本
# 一键启动 Godot 客户端

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CLIENT_DIR="$SCRIPT_DIR/client"
GODOT_EXE="$HOME/tool/Godot/Godot_v4.6.2-stable_win64.exe"

# Windows 路径转换
if [[ "$OSTYPE" == "msys" ]]; then
    GODOT_EXE="D:/tool/Godot/Godot_v4.6.2-stable_win64.exe"
fi

echo "=== Agentora Client 启动脚本 ==="
echo "项目目录：$SCRIPT_DIR"
echo "客户端目录：$CLIENT_DIR"

# 检查 DLL 是否存在
if [ ! -f "$CLIENT_DIR/bin/agentora_bridge.dll" ]; then
    echo "警告：GDExtension DLL 不存在，正在构建..."
    cd "$SCRIPT_DIR"
    cargo build -p agentora-bridge
    cp "target/debug/agentora_bridge.dll" "$CLIENT_DIR/bin/"
    cp "target/debug/agentora_bridge.pdb" "$CLIENT_DIR/bin/"
    echo "DLL 已复制到 $CLIENT_DIR/bin/"
fi

# 检查 Godot 是否存在
if [ ! -f "$GODOT_EXE" ]; then
    echo "错误：未找到 Godot 可执行文件：$GODOT_EXE"
    exit 1
fi

echo "启动 Godot..."
cd "$CLIENT_DIR"

# 启动模式选择
if [ "$1" == "--editor" ]; then
    echo "以编辑器模式启动..."
    "$GODOT_EXE" --path . --editor
elif [ "$1" == "--build" ]; then
    echo "构建 Release 版本..."
    cd "$SCRIPT_DIR"
    cargo build --release -p agentora-bridge
    cp "target/release/agentora_bridge.dll" "$CLIENT_DIR/bin/"
    echo "Release DLL 已复制"
else
    echo "以运行模式启动..."
    "$GODOT_EXE" --path .
fi

echo "完成"
