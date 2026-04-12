#!/bin/bash
# 编译 bridge crate 并复制 DLL 到 Godot 项目
# 用法: bash scripts/build-bridge.sh [--release]

set -e

PROFILE="debug"
FLAGS=""

if [[ "$1" == "--release" ]]; then
    PROFILE="release"
    FLAGS="--release"
fi

echo ">>> 编译 agentora-bridge ($PROFILE)..."
cargo build -p agentora-bridge $FLAGS

echo ">>> 复制产物到 client/bin/..."
cp -f "target/$PROFILE/agentora_bridge.dll" "client/bin/agentora_bridge.dll"

# 复制 PDB 用于本地调试（不提交到 git）
if [[ -f "target/$PROFILE/agentora_bridge.pdb" ]]; then
    cp -f "target/$PROFILE/agentora_bridge.pdb" "client/bin/agentora_bridge.pdb"
fi

echo ">>> 完成！client/bin/agentora_bridge.dll 已更新"
