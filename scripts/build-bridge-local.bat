@echo off
REM 编译带 local-inference feature 的 bridge
REM 需要先安装 LLVM: https://github.com/llvm/llvm-project/releases

setlocal

set "REPO_DIR=%~dp0.."
cd /d %REPO_DIR%

REM 设置 LLVM 路径（根据实际安装路径调整）
set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"

echo [INFO] 编译 agentora-bridge (local-inference feature)...
echo [INFO] LIBCLANG_PATH=%LIBCLANG_PATH%

cargo build -p agentora-bridge --features local-inference
if %errorlevel% neq 0 (
    echo [ERROR] 编译失败！请确保 LLVM 已安装
    echo [HINT] 下载 LLVM: https://github.com/llvm/llvm-project/releases
    exit /b %errorlevel%
)

echo [INFO] 复制产物到 client\bin\...
copy /Y "target\debug\agentora_bridge.dll" "client\bin\agentora_bridge.dll"

echo [INFO] 完成！local-inference 模式已启用