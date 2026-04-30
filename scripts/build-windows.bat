@echo off
REM 打包 Windows exe：编译 bridge -> 创建 build 目录 -> 复制配置 -> Godot 导出
REM 用法: build-windows.bat

setlocal
set "REPO_DIR=%~dp0.."
cd /d %REPO_DIR%

echo [1/4] 编译 agentora-bridge (release)...
call cargo build -p agentora-bridge --release
if %errorlevel% neq 0 exit /b %errorlevel%

echo [2/4] 创建 build 目录结构...
rmdir /s /q build 2>nul
mkdir build
mkdir build\config
mkdir build\worldseeds
mkdir build\client
xcopy /s /e /y /i client\bin build\client\bin

echo [3/4] 复制配置文件...
copy /Y config\sim.toml build\config\
copy /Y config\log.toml build\config\
copy /Y config\llm.toml build\config\
copy /Y config\user_config.toml build\config\ 2>nul
copy /Y worldseeds\default.toml build\worldseeds\

echo [4/4] Godot 导出...
cd client
godot --headless --export-release "Windows Desktop" ../build/agentora_windows.exe
if %errorlevel% neq 0 exit /b %errorlevel%

echo.
echo [完成] build\agentora_windows.exe 已就绪
echo 运行: cd build ^&^& agentora_windows.exe
