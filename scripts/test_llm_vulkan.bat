@echo off
set LIBCLANG_PATH=D:\Program Files\LLVM\bin
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" > nul 2>&1
cd /d D:\work\code\rust\agentora

echo === Testing Vulkan Backend ===
cargo clean -p llama-cpp-sys-2 2>nul
cargo clean -p agentora-ai 2>nul
cargo test --test test_llm_inference -p agentora-ai --features vulkan -- --ignored 2>&1