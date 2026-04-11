@echo off
REM Agentora Client 启动脚本 (Windows 批处理版本)
REM 一键启动 Godot 客户端

setlocal enabledelayedexpansion

echo === Agentora Client 启动脚本 ===

set SCRIPT_DIR=%~dp0
set CLIENT_DIR=%SCRIPT_DIR%client
set GODOT_EXE=D:\tool\Godot_v4.6.2-stable_win64.exe\Godot_v4.6.2-stable_win64.exe

echo 项目目录：%SCRIPT_DIR%
echo 客户端目录：%CLIENT_DIR%

REM 检查 DLL 是否存在
if not exist "%CLIENT_DIR%\bin\agentora_bridge.dll" (
    echo 警告：GDExtension DLL 不存在，正在构建...
    cd /d %SCRIPT_DIR%
    cargo build -p agentora-bridge
    copy /Y "target\debug\agentora_bridge.dll" "%CLIENT_DIR%\bin\"
    copy /Y "target\debug\agentora_bridge.pdb" "%CLIENT_DIR%\bin\"
    echo DLL 已复制到 %CLIENT_DIR%\bin\
)

REM 检查 Godot 是否存在
if not exist "%GODOT_EXE%" (
    echo 错误：未找到 Godot 可执行文件：%GODOT_EXE%
    pause
    exit /b 1
)

echo 启动 Godot...
cd /d %CLIENT_DIR%

REM 启动模式选择
if "%1"=="--editor" (
    echo 以编辑器模式启动...
    "%GODOT_EXE%" --path .
) else if "%1"=="--build" (
    echo 构建 Release 版本...
    cd /d %SCRIPT_DIR%
    cargo build --release -p agentora-bridge
    copy /Y "target\release\agentora_bridge.dll" "%CLIENT_DIR%\bin\"
    echo Release DLL 已复制
) else (
    echo 以运行模式启动...
    "%GODOT_EXE%" --path .
)

echo 完成
pause
