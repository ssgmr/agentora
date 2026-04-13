#!/bin/bash
# 编译 bridge crate 并复制动态库到 Godot 项目
# 用法: bash scripts/build-bridge.sh [--release]

set -e

# 确保 cargo 在 PATH 中
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
[[ -d "/opt/homebrew/opt/rustup/bin" ]] && export PATH="/opt/homebrew/opt/rustup/bin:$PATH"

PROFILE="debug"
FLAGS=""

if [[ "$1" == "--release" ]]; then
    PROFILE="release"
    FLAGS="--release"
fi

# 项目根目录（脚本在 scripts/ 下，向上一级就是根目录）
REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"

# 检查动态库扩展名
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

echo ">>> 编译 agentora-bridge ($PROFILE)..."
cd "$REPO_DIR"
cargo build -p agentora-bridge $FLAGS

echo ">>> 复制产物到 client/bin/..."
cp -f "target/$PROFILE/$LIB_NAME" "client/bin/"

echo ">>> 完成！client/bin/$LIB_NAME 已更新"