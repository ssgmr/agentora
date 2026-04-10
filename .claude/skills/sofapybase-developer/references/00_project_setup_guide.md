# 项目初始化指南

本文档介绍从脚手架创建项目后进行本地开发环境的初始化设置。

## 1. 安装 uv

uv 是一个快速的 Python 包管理工具，用于替代传统的 pip 和 virtualenv。

执行以下命令检查 uv 是否已安装：

```bash
uv --version
```

若有版本号输出，说明已安装，继续下一步。若未安装，执行：

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

安装完成后，重新打开终端或执行 `source ~/.zshrc` 使 uv 生效。

## 2. 安装并重启 Layotto(Mosn)

Layotto(Mosn) 是服务网格运行时，为应用提供服务调用、配置管理等能力。

检查 mosn 运行状态：

```bash
meshboot status -m binary
```

若输出 `{"status":"UP","components":{...}}`，说明 Mosn 已运行，需要重启以加载当前应用名配置。若未运行，参考 [Mosn 安装指南](01_mosn_guide.md) 进行安装。

## 3. 创建并激活虚拟环境

虚拟环境用于隔离项目依赖，避免与系统 Python 环境冲突。

在项目根目录（与 requirements.txt 同级）执行：

```bash
uv venv --python 3.12
source .venv/bin/activate
```

## 4. 安装项目依赖

```bash
uv pip install -r requirements.txt
```

若上述命令安装失败，可使用以下备选方式：

```bash
python -m ensurepip --upgrade
python -m pip install -r requirements.txt -i https://artifacts.antgroup-inc.cn/simple/
```

## 5. 下一步

完成项目初始化后，阅读 [配置机制说明](02_config_reference.md) 了解详细配置。根据开发需求参考：[Web 开发](05_web_development_guide.md)、[SOFA 开发](03_sofa_development_guide.md)、[MCP 开发](04_mcp_development_guide.md)、[中间件配置](10_middleware_usage_guide.md)。