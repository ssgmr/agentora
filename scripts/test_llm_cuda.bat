@echo off
set LIBCLANG_PATH=D:\Program Files\LLVM\bin
set CUDA_PATH=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.4
set CUDACXX=%CUDA_PATH%\bin\nvcc.exe
set PATH=%CUDA_PATH%\bin;%PATH%
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" > nul 2>&1
cd /d D:\work\code\rust\agentora

echo === Testing CUDA Backend ===
echo CUDA_PATH=%CUDA_PATH%
echo CUDACXX=%CUDACXX%
cargo clean -p llama-cpp-sys-2 2>nul
cargo clean -p agentora-ai 2>nul
cargo test --test test_llm_inference -p agentora-ai --features cuda -- --ignored 2>&1