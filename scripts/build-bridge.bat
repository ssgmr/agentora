@echo off
REM 编译 bridge crate 并复制 DLL 到 Godot 项目
REM 用法: build-bridge.bat 或 build-bridge.bat release

setlocal

REM 项目根目录（脚本在 scripts\ 下，向上一级）
set "REPO_DIR=%~dp0.."

if "%1"=="release" (
    set PROFILE=release
    set FLAGS=--release
) else (
    set PROFILE=debug
    set FLAGS=
)

echo >>> 编译 agentora-bridge (%PROFILE%)...
cd /d %REPO_DIR%
call cargo build -p agentora-bridge %FLAGS%
if %errorlevel% neq 0 exit /b %errorlevel%

echo >>> 复制产物到 client\bin\...
copy /Y "target\%PROFILE%\agentora_bridge.dll" "client\bin\agentora_bridge.dll"
if exist "target\%PROFILE%\agentora_bridge.pdb" (
    copy /Y "target\%PROFILE%\agentora_bridge.pdb" "client\bin\agentora_bridge.pdb"
)

echo >>> 完成！client\bin\agentora_bridge.dll 已更新