@echo off
REM P2P Dual Node Test Script

set PROJECT_ROOT=%~dp0..
set CLIENT_DIR=%PROJECT_ROOT%\client

echo === Agentora P2P Dual Node Test ===
echo Project Root: %PROJECT_ROOT%

REM Check godot command
where godot >nul 2>&1
if errorlevel 1 (
    echo ERROR: godot command not found in PATH
    pause
    exit /b 1
)

echo.
echo Starting Node A (Seed Node, Port 4001)...
start "Agentora-NodeA" cmd /c "set AGENTORA_SIM_CONFIG=../config/sim_node_a.toml && godot --path %CLIENT_DIR%"

echo Waiting 3 seconds...
ping -n 4 127.0.0.1 >nul

echo.
echo Starting Node B (Client Node, Port 4002)...
start "Agentora-NodeB" cmd /c "set AGENTORA_SIM_CONFIG=../config/sim_node_b.toml && godot --path %CLIENT_DIR%"

echo.
echo Dual nodes started! Check the two Godot windows.
echo.
echo Test steps:
echo   1. In NodeB, click P2P button
echo   2. Enter: /ip4/127.0.0.1/tcp/4001
echo   3. Click Connect
echo.
pause