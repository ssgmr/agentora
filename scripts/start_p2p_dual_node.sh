#!/bin/bash
# P2P 双节点端到端测试启动脚本
# 使用环境变量 AGENTORA_SIM_CONFIG 指定配置文件

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CLIENT_DIR="$PROJECT_ROOT/client"
LOGS_DIR="$PROJECT_ROOT/logs"

echo "=== Agentora P2P 双节点测试 ==="
echo "项目根目录: $PROJECT_ROOT"

# 检查 Godot 是否可用
if ! command -v godot &> /dev/null; then
    echo "错误: Godot 4 未找到"
    echo "请确保 godot 命令在 PATH 中"
    exit 1
fi

# 检查配置文件是否存在
if [ ! -f "$PROJECT_ROOT/config/sim_node_a.toml" ]; then
    echo "错误: config/sim_node_a.toml 不存在"
    exit 1
fi
if [ ! -f "$PROJECT_ROOT/config/sim_node_b.toml" ]; then
    echo "错误: config/sim_node_b.toml 不存在"
    exit 1
fi

# 检查 bridge DLL 是否存在
if [ ! -f "$CLIENT_DIR/bin/agentora_bridge.dll" ]; then
    echo "警告: bridge DLL 不存在，正在编译..."
    cargo build -p agentora-bridge
    cp target/debug/agentora_bridge.dll "$CLIENT_DIR/bin/"
fi

# 确保日志目录存在
mkdir -p "$LOGS_DIR"

# 清理旧日志
rm -f "$LOGS_DIR/node_a.log" "$LOGS_DIR/node_b.log"

echo ""
echo "启动节点 A（种子节点，端口 4001）..."
echo "配置文件: config/sim_node_a.toml"

# Windows Git Bash 环境变量导出方式
export AGENTORA_SIM_CONFIG="$PROJECT_ROOT/config/sim_node_a.toml"
godot --path "$CLIENT_DIR" --rendering-driver opengl3 > "$LOGS_DIR/node_a.log" 2>&1 &
PID_A=$!
echo "节点 A PID: $PID_A"
unset AGENTORA_SIM_CONFIG

sleep 5  # 等待节点 A 完全启动

echo ""
echo "启动节点 B（连接节点，端口 4002）..."
echo "配置文件: config/sim_node_b.toml"

export AGENTORA_SIM_CONFIG="$PROJECT_ROOT/config/sim_node_b.toml"
godot --path "$CLIENT_DIR" --rendering-driver opengl3 > "$LOGS_DIR/node_b.log" 2>&1 &
PID_B=$!
echo "节点 B PID: $PID_B"
unset AGENTORA_SIM_CONFIG

echo ""
echo "双节点已启动！"
echo "日志目录: $LOGS_DIR/"
echo "  - node_a.log: 种子节点日志"
echo "  - node_b.log: 连接节点日志"
echo ""
echo "验证步骤:"
echo "  1. 在节点 B 的 P2P 面板中查看 Peer ID（应为 local_4002）"
echo "  2. 查看已连接节点列表是否显示节点 A（Peer ID: local_4001）"
echo "  3. 检查订阅的 Topic（应有 world_events 和 region_0）"
echo ""
echo "按 Ctrl+C 停止所有节点..."

trap "echo ''; echo '正在停止节点...'; kill $PID_A $PID_B 2>/dev/null || true; echo '节点已停止'; exit 0" INT TERM

# 等待进程（保持脚本运行）
wait $PID_A $PID_B 2>/dev/null || true