@echo off
REM Agentora Client 启动脚本 (Windows 批处理版本)
REM 一键启动 Godot 客户端

setlocal enabledelayedexpansion

echo === Agentora Client 启动脚本 ===

REM 项目根目录（脚本在 scripts\ 下，向上一级）
set "REPO_DIR=%~dp0.."
set "CLIENT_DIR=%REPO_DIR%\client"

echo 仓库目录：%REPO_DIR%
echo 客户端目录：%CLIENT_DIR%

REM 检查 Godot 是否可用
where godot >nul 2>nul
if errorlevel 1 (
    echo 错误：未找到 godot 命令，请确保 Godot 已安装并添加到 PATH
    pause
    exit /b 1
)

REM 检查 DLL 是否存在
if not exist "%CLIENT_DIR%\bin\agentora_bridge.dll" (
    echo 警告：GDExtension DLL 不存在，正在构建...
    cd /d %REPO_DIR%
    cargo build -p agentora-bridge
    copy /Y "target\debug\agentora_bridge.dll" "%CLIENT_DIR%\bin\"
    if exist "target\debug\agentora_bridge.pdb" (
        copy /Y "target\debug\agentora_bridge.pdb" "%CLIENT_DIR%\bin\"
    )
    echo DLL 已复制到 %CLIENT_DIR%\bin\
)

echo 启动 Godot...
cd /d %CLIENT_DIR%

REM 启动模式选择
if "%1"=="--editor" (
    echo 以编辑器模式启动...
    godot --path . --editor
) else if "%1"=="--build" (
    echo 构建 Release 版本...
    cd /d %REPO_DIR%
    cargo build --release -p agentora-bridge
    copy /Y "target\release\agentora_bridge.dll" "%CLIENT_DIR%\bin\"
    if exist "target\release\agentora_bridge.pdb" (
        copy /Y "target\release\agentora_bridge.pdb" "%CLIENT_DIR%\bin\"
    )
    echo Release DLL 已复制
) else (
    echo 以运行模式启动...
    godot --path .
)

echo 完成
pause