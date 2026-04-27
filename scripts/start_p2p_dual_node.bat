@echo off
REM P2P 双节点端到端测试启动脚本 (Windows)
REM 使用环境变量 AGENTORA_SIM_CONFIG 指定配置文件

setlocal enabledelayedexpansion

set PROJECT_ROOT=%~dp0..
set CLIENT_DIR=%PROJECT_ROOT%\client
set LOGS_DIR=%PROJECT_ROOT%\logs

echo === Agentora P2P 双节点测试 ===
echo 项目根目录: %PROJECT_ROOT%

REM 检查配置文件是否存在
if not exist "%PROJECT_ROOT%\config\sim_node_a.toml" (
    echo 错误: config\sim_node_a.toml 不存在
    exit /b 1
)
if not exist "%PROJECT_ROOT%\config\sim_node_b.toml" (
    echo 错误: config\sim_node_b.toml 不存在
    exit /b 1
)

REM 检查 bridge DLL 是否存在
if not exist "%CLIENT_DIR%\bin\agentora_bridge.dll" (
    echo 警告: bridge DLL 不存在，请先运行 cargo bridge
    exit /b 1
)

REM 确保日志目录存在
if not exist "%LOGS_DIR%" mkdir "%LOGS_DIR%"

echo.
echo 启动节点 A（种子节点，端口 4001）...
set AGENTORA_SIM_CONFIG=../config/sim_node_a.toml
start "NodeA" godot --path "%CLIENT_DIR%" --rendering-driver opengl3
timeout /t 3 /nobreak > /dev/null

echo.
echo 启动节点 B（连接节点，端口 4002）...
set AGENTORA_SIM_CONFIG=../config/sim_node_b.toml
start "NodeB" godot --path "%CLIENT_DIR%" --rendering-driver opengl3

echo.
echo 双节点已启动！
echo.
echo 验证步骤:
echo   1. 在节点 B 的 P2P 面板中输入种子地址: /ip4/127.0.0.1/tcp/4001
echo   2. 点击连接按钮
echo   3. 检查已连接 peers 列表是否显示节点 A
echo   4. 观察节点 A 的 Agent 是否在节点 B 中显示
echo.
echo 关闭此窗口不会停止节点，请在任务管理器中结束 Godot 进程

endlocal
