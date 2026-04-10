---
name: sofapybase-developer
description: "基于 sofapy (ant-sofapy-base) 框架进行应用开发。支持项目初始化、Web(FastAPI)、SOFA服务、MCP工具及中间件配置。触发关键词：初始化项目, 开发Web接口, 开发SOFA服务, 创建MCP工具, 配置中间件, sofapy, ant-sofapy-base"
allowed-tools: Read, Edit, Write, Bash, Grep, Glob
---

# Sofapy 开发助手

基于 ant-sofapy-base 框架进行应用开发，支持 Web（FastAPI）、SOFA RPC、MCP 三种应用类型，支持接入蚂蚁内部中间件。

## 项目结构

标准项目结构如下：

```
{appname}/                      # 项目根目录
├── conf/                       # Docker 配置
│   └── docker/                 # Dockerfile, nginx.conf 等
├── {appname}/                  # 应用代码目录
│   ├── configs/                # 配置文件目录
│   │   ├── application.yaml    # 主配置（必需）
│   │   └── application-{env}.yaml
│   ├── main.py                 # 应用入口
│   └── servers/                # 服务实现（至少包含一种服务类型）
│       ├── mcp/                # MCP 服务
│       ├── sofa/               # SOFA RPC 服务
│       └── web/                # Web 服务
├── .claude/                    # Claude Code 配置
├── .codefuse/                  # CodeFuse 配置
├── requirements.txt            # Python 依赖
├── LEGAL.md                    # 法律免责声明
└── README.md
```

## 开发规范

新增功能时需遵循以下流程：先了解需求，明确需求背景与使用场景；再设计方案，提出 API 设计、实现思路、模块位置等方案；待用户确认后方可编写代码；Feature 开发完成后，需同步更新用户项目的 README.md。在用户确认前不要编写实现代码。

## 开发工作流

### 1. 项目初始化

详见 [项目初始化指南](references/00_project_setup_guide.md)。

### 2. Web 接口开发（若需要）

详见 [Web 开发指南](references/05_web_development_guide.md)。

### 3. SOFA 服务开发（若需要）

发布 TR 服务详见 [SOFA 开发指南](references/03_sofa_development_guide.md)，调用 TR 服务详见 [TR 调用指南](references/11_tr_usage_guide.md)，使用其他中间件（ZCache、DRM、Mist、ZDAS、DDSOSS、Flowcontrol、分布式锁、Maya、消息队列等）详见 [中间件使用指南](references/10_middleware_usage_guide.md)。

### 4. MCP 开发（若需要）

发布 MCP Server 详见 [MCP 开发指南](references/04_mcp_development_guide.md)，调用 MCP Server 详见 [MCP Client 使用指南](references/06_mcp_client_usage_guide.md)。

### 5. 配置中间件、日志与链路追踪（若需要）

详见 [中间件使用指南](references/10_middleware_usage_guide.md)、[日志使用指南](references/07_logger_usage_guide.md)、[Tracer 使用指南](references/08_tracer_usage_guide.md)。

### 6. 服务启动与停止

在项目根目录（与 requirements.txt 同级）执行：

```bash
source .venv/bin/activate   # 激活虚拟环境
cd <应用名>                  # 进入应用目录（如 cd sofapyapp）
python main.py              # 启动服务
```

测试完成后，通过 `ps aux | grep main.py` 查找进程，再通过 `kill <pid>` 终止进程。若后台运行，可通过 `python main.py 2>&1 &` 启动，通过 `pkill -f "python main.py"` 停止。

### 7. 日志排查

遇到问题时，可通过日志定位原因。详见 [日志排查指南](references/21_log_troubleshooting_guide.md)。

应用业务日志位于 `~/logs/<应用名>/`，推荐优先查看 `common-error.log`。链路追踪日志位于 `~/logs/tracelog/`，可通过 TraceId 追踪完整调用链路。

### 8. 框架迁移

从 `sofapy` 迁移到 `ant-sofapy-base`，详见 [迁移指南](references/30_migration_guide.md)。

### 9. 常见问题

详见 [常见问题汇总](references/20_faq_guide.md)。