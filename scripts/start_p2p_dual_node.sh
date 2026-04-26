#!/bin/bash
# P2P 双节点端到端测试启动脚本
# 使用环境变量 AGENTORA_SIM_CONFIG 指定配置文件

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CLIENT_DIR="$PROJECT_ROOT/client"

echo "=== Agentora P2P 双节点测试 ==="
echo "项目根目录: $PROJECT_ROOT"

# 检查 Godot 是否可用
if ! command -v godot &> /dev/null; then
    echo "错误: Godot 4 未找到"
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
    bash "$PROJECT_ROOT/scripts/build-bridge.sh"
fi

echo ""
echo "启动节点 A（种子节点，端口 4001）..."
AGENTORA_SIM_CONFIG="../config/sim_node_a.toml" \
    godot --path "$CLIENT_DIR" --rendering-driver opengl3 \
    > "$PROJECT_ROOT/logs/node_a.log" 2>&1 &
PID_A=$!
echo "节点 A PID: $PID_A"

sleep 3  # 等待节点 A 启动

echo ""
echo "启动节点 B（连接节点，端口 4002）..."
AGENTORA_SIM_CONFIG="../config/sim_node_b.toml" \
    godot --path "$CLIENT_DIR" --rendering-driver opengl3 \
    > "$PROJECT_ROOT/logs/node_b.log" 2>&1 &
PID_B=$!
echo "节点 B PID: $PID_B"

echo ""
echo "双节点已启动！"
echo "日志目录: $PROJECT_ROOT/logs/"
echo "  - node_a.log: 种子节点日志"
echo "  - node_b.log: 连接节点日志"
echo ""
echo "验证步骤:"
echo "  1. 在节点 B 的 P2P 面板中输入种子地址: /ip4/127.0.0.1/tcp/4001"
echo "  2. 点击连接按钮"
echo "  3. 检查已连接 peers 列表是否显示节点 A"
echo "  4. 观察节点 A 的 Agent 是否在节点 B 中显示"
echo ""
echo "按 Ctrl+C 停止所有节点..."

trap "kill $PID_A $PID_B 2>/dev/null; echo '节点已停止'; exit 0" INT TERM

# 等待进程
wait