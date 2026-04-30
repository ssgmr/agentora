@echo off
set LIBCLANG_PATH=D:\Program Files\LLVM\bin
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" > nul 2>&1
cd /d D:\work\code\rust\agentora

echo === Building agentora-ai with local-inference ===
cargo build -p agentora-ai --features local-inference 2>&1

echo.
echo === Running LLM inference test ===
cargo test --test test_llm_inference -p agentora-ai --features local-inference -- --ignored 2>&1