# 安装 oneapi-codegen 代码生成工具

本指南仅需执行一次。若后续重新安装或遇到环境问题，可参考本文档排查。

## 检查是否已安装

```bash
oneapi-codegen -v
```

若已安装（显示版本号），可跳过安装步骤。

## 安装 Node.js 和 npm

先检查是否已安装：

```bash
node -v
npm -v
```

若已安装（显示版本号），可跳过 Node.js 和 npm 安装步骤。

若未安装，执行以下命令：

```bash
# 安装 Homebrew
curl -o- https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh | bash

# 安装 Node.js
brew install node@24

# 验证版本
node -v
npm -v
```

## 安装 tnpm

```bash
# 创建 npm 全局目录
mkdir -p $HOME/.npm-global
npm config set prefix $HOME/.npm-global
echo 'export PATH="$HOME/.npm-global/bin:$PATH"' >> $HOME/.zshrc
source $HOME/.zshrc

# 安装 tnpm 10.x
npm i tnpm@10 -g --registry=https://registry.antgroup-inc.cn

# 验证版本
tnpm -v
```

## 安装 oneapi-codegen

```bash
npm i -g @alipay/oneapi-codegen-sdk

# 验证版本 >= 5.10.0
oneapi-codegen -v
```

## 常见问题：环境依赖缺失

以下问题通常在执行 `oneapi-codegen layotto publish --lang=python` 命令时出现。

### 问题一：Java 运行环境缺失

错误信息：

```
The operation couldn't be completed. Unable to locate a Java Runtime.
Please visit http://www.java.com for information on installing Java.

[oneapi-codegen] AssertionError [ERR_ASSERTION]: 请确认 Java 是否正确的安装并正确的配置 JAVA_HOME 环境变量
```

原因：`oneapi-codegen` 工具链中处理 JAR 依赖 Java 环境，当前系统未安装 Java (JDK)，或未正确配置 `JAVA_HOME` 环境变量。

解决方案：通过 Homebrew 安装 JDK。安装包约 200MB，配置国内镜像源后仍需要 5 分钟左右。需询问用户是否同意执行安装，用户同意后按顺序执行以下命令：

```bash
# 配置 Homebrew 镜像源（清华镜像，加速下载）
export HOMEBREW_API_DOMAIN="https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles/api"
export HOMEBREW_BOTTLE_DOMAIN="https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles"
export HOMEBREW_BREW_GIT_REMOTE="https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git"
export HOMEBREW_CORE_GIT_REMOTE="https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/homebrew-core.git"

# 安装 OpenJDK 17（耗时5分钟左右，请提示耐心等待）
brew install openjdk@17
```

安装完成后，需要创建系统链接。由于需要输入 Mac 开机密码，请让用户自行执行以下命令：

```bash
# 创建系统链接（sudo权限需要用户输入mac开机密码）
sudo ln -sfn /opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk /Library/Java/JavaVirtualMachines/openjdk-17.jdk
```

最后执行以下命令完成环境变量配置：

```bash
# 写入配置文件（永久生效）
echo 'export JAVA_HOME=$(/usr/libexec/java_home)' >> ~/.zshrc
echo 'export PATH="$JAVA_HOME/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### 问题二：找不到 Maven 模块

错误信息：`[oneapi-codegen] Error: Cannot find module '@alipay/maven'`

原因：Node.js 找不到 `@alipay/maven` 模块，需配置全局模块搜索路径。

解决方案：添加 NODE_PATH 环境变量：

```bash
echo 'export NODE_PATH=$(tnpm root -g):$NODE_PATH' >> ~/.zshrc
source ~/.zshrc
```