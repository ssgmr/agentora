#!/bin/bash
# Agentora 多节点本地测试启动脚本
# 支持 2-10 节点的本地测试环境
# 新增：DCUtR/AutoNAT 混合穿透测试支持

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CLIENT_DIR="$PROJECT_ROOT/client"

# 默认节点数量，可通过参数覆盖
NODE_COUNT=${1:-2}

if [ "$NODE_COUNT" -lt 2 ] || [ "$NODE_COUNT" -gt 10 ]; then
    echo "错误: 节点数量必须在 2-10 之间"
    exit 1
fi

echo "=== Agentora 多节点启动脚本 ==="
echo "项目根目录: $PROJECT_ROOT"
echo "节点数量: $NODE_COUNT"
echo "测试模式: libp2p 0.56 + DCUtR + AutoNAT"

# 检查 Godot 是否可用
if ! command -v godot &> /dev/null; then
    echo "警告: Godot 4 未找到，将仅运行 Rust 测试"
    echo "运行 cargo 测试..."
    cd "$PROJECT_ROOT"
    cargo test -p agentora-network --test multi_node_tests -- --nocapture
    exit $?
fi

# 清理旧节点数据
if [ -d "$PROJECT_ROOT/.nodes" ]; then
    echo "清理旧节点数据..."
    rm -rf "$PROJECT_ROOT/.nodes"
fi

# 创建节点配置目录
for i in $(seq 1 $NODE_COUNT); do
    NODE_DIR="$PROJECT_ROOT/.nodes/node_$i"
    mkdir -p "$NODE_DIR"

    # 复制 WorldSeed 配置
    cp "$PROJECT_ROOT/worldseeds/default.toml" "$NODE_DIR/WorldSeed.toml"

    # 为每个节点生成唯一的密钥路径
    echo "node_key_path=$NODE_DIR/peer_key.bin" > "$NODE_DIR/config.env"
done

echo ""
echo "启动节点..."

# 启动种子节点（最后一个节点作为种子）
SEED_NODE=$NODE_COUNT
echo "启动节点 $SEED_NODE (种子节点)..."
godot --path "$CLIENT_DIR" --rendering-driver opengl3 \
    -- --node-id=$SEED_NODE --seed-mode=true \
    > "$PROJECT_ROOT/.nodes/node_$SEED_NODE/log.txt" 2>&1 &
SEED_PID=$!
sleep 2

# 启动其他节点，连接到种子节点
for i in $(seq 1 $((NODE_COUNT - 1))); do
    echo "启动节点 $i (连接到种子节点 $SEED_NODE)..."
    godot --path "$CLIENT_DIR" --rendering-driver opengl3 \
        -- --node-id=$i --seed-peer=localhost:4001 \
        > "$PROJECT_ROOT/.nodes/node_$i/log.txt" 2>&1 &
    eval "PID_$i=$!"
    sleep 1
done

echo ""
echo "节点已启动:"
echo "  - 种子节点 (Node $SEED_NODE) PID: $SEED_PID"
for i in $(seq 1 $((NODE_COUNT - 1))); do
    eval "echo '  - Node $i PID: \$PID_$i'"
done
echo ""
echo "日志目录: $PROJECT_ROOT/.nodes/"
echo ""
echo "运行 Rust 网络测试..."
cd "$PROJECT_ROOT"
cargo test -p agentora-network --test multi_node_tests -- --nocapture

echo ""
echo "按 Ctrl+C 停止所有节点..."

# 构建停止命令
STOP_PIDS="$SEED_PID"
for i in $(seq 1 $((NODE_COUNT - 1))); do
    eval "STOP_PIDS=\"\$STOP_PIDS \$PID_$i\""
done

trap "kill $STOP_PIDS 2>/dev/null; echo '节点已停止'; exit 0" INT TERM

wait
