# DLL 打包说明

本文档记录各平台 llama.cpp 预编译 DLL 打包流程。

## 前置要求

- libclang 安装（用于 bindgen）
  - Windows: 安装 LLVM 或 Visual Studio with Clang
  - macOS: Xcode Command Line Tools
  - Linux: `apt install libclang-dev`

- Vulkan SDK（用于 Vulkan 后端）
  - Windows: https://vulkan.lunarg.com/sdk/home#windows
  - Linux: `apt install vulkan-sdk`

- CUDA Toolkit（可选，用于 CUDA 后端）
  - https://developer.nvidia.com/cuda-downloads

## 5.1 Windows Vulkan DLL 打包

### 编译步骤

```bash
# 1. 设置环境变量
set LIBCLANG_PATH=C:\Program Files\LLVM\bin

# 2. 编译 agentora-bridge with Vulkan feature
cargo build -p agentora-bridge --features vulkan --release

# 3. 编译 llama-cpp-2（手动下载预编译 DLL）
# 从 https://github.com/ggerganov/llama.cpp/releases 下载
# 需要: ggml.dll, ggml-vulkan.dll, llama.dll

# 4. 复制 DLL 到 client/bin/
copy target\release\agentora_bridge.dll client\bin\
copy llama_cpp_dlls\ggml.dll client\bin\
copy llama_cpp_dlls\ggml-vulkan.dll client\bin\
```

### Vulkan 系统依赖

- `vulkan-1.dll`: 由 Vulkan SDK 或显卡驱动提供
- 用户需安装 Vulkan Runtime（通常显卡驱动自带）

## 5.2 Windows CUDA DLL 打包（可选）

### 编译步骤

```bash
# 1. 设置 CUDA 环境变量
set CUDA_PATH=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.4

# 2. 编译 with CUDA feature
cargo build -p agentora-bridge --features cuda --release

# 3. 复制 CUDA DLL
copy llama_cpp_dlls\ggml-cuda.dll client\bin\
copy llama_cpp_dlls\cudart64_*.dll client\bin\
```

### CUDA 系统依赖

- CUDA Runtime: 用户需安装 CUDA Toolkit 或 NVIDIA 驱动
- CUDA 12.x 兼容 NVIDIA RTX 系列 GPU

## 5.3 macOS Metal DLL 打包

### 编译步骤

```bash
# 1. 设置 libclang（Xcode 自带）
export LIBCLANG_PATH=/Library/Developer/CommandLineTools/usr/bin

# 2. 编译 with Metal feature
cargo build -p agentora-bridge --features metal --release

# 3. Metal 是 macOS 系统自带，无需额外 DLL
# llama-cpp-2 会自动链接 Metal.framework
```

### Metal 系统依赖

- Metal Framework: macOS/iOS 系统自带
- 无需用户安装额外组件

## 5.4 Android Vulkan DLL 打包

### 编译步骤

```bash
# 1. 设置 Android NDK
export ANDROID_NDK_HOME=/path/to/android-ndk

# 2. 编译 llama.cpp for Android
# 参考 llama.cpp/scripts/build-android.sh

# 3. 复制 .so 文件
cp llama_cpp_android/libllama.so client/android/libs/arm64-v8a/
cp llama_cpp_android/libggml-vulkan.so client/android/libs/arm64-v8a/
```

### Android Vulkan 依赖

- Android 7.0+ 自带 Vulkan 支持
- 需设备支持 Vulkan（现代 Android 设备基本都支持）

## 5.5 Godot Export 配置

### Windows Export

在 `client/project.godot` 添加:

```ini
[export.Windows Desktop]
binary = "agentora_windows.exe"
include = [
  "bin/*.dll"
]
```

### macOS Export

```ini
[export.MacOS]
binary = "Agentora.app"
include = [
  "bin/*.dylib"
]
```

### Android Export

```ini
[export.Android]
binary = "agentora.apk"
include = [
  "android/libs/**/*.so"
]
```

## 预编译 DLL 来源

推荐从以下渠道获取预编译 DLL：

1. **llama.cpp Releases**: https://github.com/ggerganov/llama.cpp/releases
2. **自行编译**: 使用 llama.cpp 的 `scripts/build_*.sh` 脚本
3. **Cargo 编译**: `cargo build --features all-gpu`（需要完整编译环境）

## 验证清单

编译完成后验证：

1. ✅ DLL 文件存在于 `client/bin/`
2. ✅ Godot 运行时正确加载 GDExtension
3. ✅ GPU 后端检测正确（`get_gpu_backend()`）
4. ✅ 模型加载进度信号发射正常
5. ✅ 推理功能正常（`generate()` 返回结果）