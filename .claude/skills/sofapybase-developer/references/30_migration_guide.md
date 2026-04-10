# Sofapy 框架迁移指南

本文档介绍如何从旧包 `sofapy` 迁移到新包 `ant-sofapy-base`。Sofapy 框架经过重构后，包名由 `sofapy` 变更为 `ant-sofapy-base`，导入路径由 `sofapy` 变更为 `sofapy_base`。本文档帮助你完成迁移。

---

## 一、迁移前准备

在开始迁移之前，需要完成以下准备工作。

### 1.1 梳理当前实现

首先梳理当前项目基于旧包名的实现情况，填写下表：

| 检查项 | 是否使用 | 说明                       |
|--------|---------|--------------------------|
| Web 服务 | □ 是 □ 否 | 是否有 Flask 路由/接口          |
| SOFA RPC 服务 | □ 是 □ 否 | 是否发布/订阅 TR 服务            |
| MCP 服务 | □ 是 □ 否 | 是否发布/调用 MCP 工具           |
| 中间件 | |                          |
| ZCache | □ 是 □ 否 | 分布式缓存                    |
| DRM | □ 是 □ 否 | 动态配置                     |
| Mist | □ 是 □ 否 | 机密管理                     |
| ZDAS | □ 是 □ 否 | 数据库访问                    |
| DDSOSS | □ 是 □ 否 | 对象存储                     |
| 消息队列 | □ 是 □ 否 | SOFAMQ/SOFAMQX/MsgBroker |
| 分布式锁 | □ 是 □ 否 | Lock                     |
| Maya | □ 是 □ 否 | AI 推理                    |

### 1.2 获取新项目模板

新包名项目需要从脚手架服务获取标准模板代码。脚手架地址：https://sofapy-start.alipay.com/

访问脚手架服务，根据上一步梳理的实现情况，选择对应的服务类型：Web 服务、SOFA RPC 服务、MCP 服务。下载生成的项目模板后，将其作为迁移的目标项目结构。

### 1.3 迁移步骤概览

完成准备工作后，按以下步骤进行迁移：首先梳理当前实现（服务类型、中间件使用情况），然后从脚手架获取新项目模板，接着更新依赖文件（requirements.txt），再将配置从字典格式迁移到 YAML 格式，之后迁移业务代码（Web/SOFA/MCP/中间件调用），最后进行测试验证。

---

## 二、核心变更概览

| 项目 | 旧包名 (sofapy) | 新包名 (ant-sofapy-base) |
|------|----------------|-------------------------|
| 包名 | `sofapy` | `ant-sofapy-base` |
| 导入路径 | `from sofapy import ...` | `from sofapy_base import ...` |
| 应用类 | `Sofapy` / `SofaApp` / `create_app()` | `SOFAPyApplication` |
| 配置方式 | 字典配置 | YAML配置 + Pydantic |
| Web框架 | Flask | FastAPI |
| RPC协议 | TR | TR + Triple |
| MCP包 | `mcp` | `ant-mcp` |

## 三、依赖变更

新包名项目使用 `requirements.txt` 进行依赖管理。旧包名使用 `pyproject.toml` 进行依赖管理。

新包名 `requirements.txt` 示例：
```text
--index-url https://pypi.antfin-inc.com/simple/

ant-sofapy-base==1.0.0
```

## 四、应用初始化变更

### 4.1 入口文件变更

旧包名入口文件：
```python
from sofapy.core.app import Sofapy, create_app

# 方式1: 使用 Sofapy 类
app = Sofapy("my-app", config_dict={
    "app_name": "my-app",
    "middlewares_config": {...}
})

# 方式2: 使用 create_app 函数
app = create_app("my-app", config_dict={...})

if __name__ == "__main__":
    app.run(host="0.0.0.0", port=8888)
```

新包名入口文件 `main.py`（脚手架已生成，一般不需修改）：
```python
#!/usr/bin/env python3
from sofapy_base.runner import run

if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description="SOFAPy Application")
    parser.add_argument("--config", "-c", type=str, default=None,
                        help="Configuration file path")

    args = parser.parse_args()
    run(config_path=args.config)
```

### 4.2 服务入口文件

新包名采用分层结构，各服务类型有独立的入口文件：

| 服务类型 | 入口文件 | 详细文档 |
|---------|---------|---------|
| SOFA RPC | `servers/sofa/app.py` + `servers/sofa/rpc/tr/*.py` | [SOFA开发指南](03_sofa_development_guide.md) |
| MCP | `servers/mcp/app.py` | [MCP开发指南](04_mcp_development_guide.md) |
| Web | `servers/web/app.py` | [Web开发指南](05_web_development_guide.md) |

### 4.3 导入路径变更

| 功能 | 旧包名 (sofapy) | 新包名 (ant-sofapy-base)                                   |
|------|----------------|---------------------------------------------------------|
| 应用启动 | `sofapy.core.app.Sofapy` | `sofapy_base.runner.run`                                |
| 应用类 | `sofapy.core.app.Sofapy` | `sofapy_base.app.application.SOFAPyApplication`         |
| 配置类 | `sofapy.core.conf.config.Config` | `sofapy_base.app.config.Config`                         |
| 日志 | `sofapy.core.logger.logger.LogManager` | `sofapy_base.logger.logger.get_logger`                  |
| Tracer | `sofapy.core.app.install_tracer_patches` | `sofapy_base.tracer.tracer.install_tracer_patches`            |
| Tracer中间件 | - | `ant_baselib.tracer.SofaTracerMiddleware`               |
| MCP工具注册 | `@app.tool()` | `from sofapy_base.mcp.registry import tool`             |
| Layotto管理器 | `sofapy.core.manager.LayottoManager` | `sofapy_base.app.layotto_manager.get_layotto_manager()` |

## 五、配置文件变更

### 5.1 配置方式变更

旧包名使用字典配置：
```python
config_dict = {
    "app_name": "my-app",
    "model": "standard",
    "enable_mcp": True,
    "middlewares_config": {
        "rpc": {
            "tr": {
                "pub": [{"service_names": ["com.example.Service"], "port": 50051}],
                "sub": ["com.example.RemoteService"]
            },
            "mcp": {
                "pub": [{"service_name": "my-mcp-service"}],
                "sub": [{"service_name": "remote-mcp"}],
                "router": "/mcp"
            }
        },
        "mist": {
            "tenant": "ALIPAY",
            "mode": "SHARE"
        }
    },
    "log_config": {
        "log_dir": "~/logs/my-app",
        "log_level": "INFO"
    }
}
app = Sofapy("my-app", config_dict=config_dict)
```

新包名使用 YAML 配置，配置文件路径为 `configs/application.yaml`：
```yaml
app_name: "sofapyapp"
enable_sidecar: true
workers: 1  # 根据需求决定是否启用多进程
health_check:
  port: 9500

log_config:
  trace_log_dir: ''
  log_level: 'INFO'
  log_dir: ''

module_config:
  rpc:
    tr:
      sub:
        - "com.alipay.basementurl.facade.URLFacadeV2:1.0"

  mcp:
    sub:
      - mcp.ant.faas.deriskMcpServerGroup.mcpAntscheduler: {"mesh_vip_address": "faasgw-pool:8080"}
      - mcp.ant.sofadoc.sofadocmcpserver: {}
    pub:
      - service_name: "sofapyserver"
        description: "created by wenxuwan"
        open_secaspect: true
        protocol: 0
        json_response: false
    start: "servers.mcp.app"

  sofa:
    start: "servers.sofa.app:app"
  web:
    port: 8888
    start: "servers.web.app:app"
```

详细配置说明参见 [配置参考](02_config_reference.md)。

## 六、RPC服务变更

### 6.1 TR协议服务

旧包名实现：
```python
from sofapy.core.app import Sofapy

app = Sofapy("my-app")
manager = app.manager.tr_manager

@manager.tr(service_name="com.example.Service", method="sayHello")
def say_hello(request):
    return {"message": "Hello"}
```

新包名实现，入口文件 `servers/sofa/app.py`：
```python
from sofapy_base.app.application import SOFAPyApplication
from sofapy_base.tracer.tracer import install_tracer_patches

app = SOFAPyApplication()
install_tracer_patches()
```

服务实现文件 `servers/sofa/rpc/tr/hello_service.py`：
```python
from layotto.ext.layotto_ext_grpc.layotto.ext.grpc.rpc import RpcRequest, SofaRpcResponse
from servers.sofa.app import app

@app.rpc(service_name='com.example.HelloFacade', method='sayHello')
def hello(request: RpcRequest) -> SofaRpcResponse:
    return SofaRpcResponse(is_error=False, app_response={"message": "Hello"})
```

详细开发说明参见 [SOFA开发指南](03_sofa_development_guide.md)。

### 6.2 Triple协议服务（新增）

新包名支持 Triple 协议：

```python
@app.rpc(
    service_name="com.example.TripleService",
    method="process",
    protocol="tri",
    unique_id="v1"
)
async def process_request(request):
    return {"result": "success"}
```

### 6.3 RPC订阅调用（TR 服务调用）

调用远程 TR 服务需要使用 oneapi-codegen 生成客户端代码。详细说明参见 [TR 调用指南](11_tr_usage_guide.md)。

## 七、MCP服务变更

### 7.1 MCP服务发布

旧包名实现：
```python
from sofapy.core.app import Sofapy

app = Sofapy("my-app")

@app.tool()
def my_tool(name: str) -> str:
    return f"Hello, {name}!"
```

新包名实现，文件路径 `servers/mcp/app.py`：
```python
from sofapy_base.mcp.registry import tool, prompt

@tool("my-mcp-service")
async def my_tool(name: str) -> str:
    """一个简单的问候工具"""
    return f"Hello, {name}!"

@prompt("my-mcp-service")
def review_code(code: str) -> str:
    """代码审查提示"""
    return f"Please review this code:\n\n{code}"
```

详细开发说明参见 [MCP开发指南](04_mcp_development_guide.md)。

### 7.2 MCP客户端调用

旧包名实现：
```python
app = Sofapy("my-app")
client = app.get_mcp_client(service_code="remote-mcp")
tools = await client.list_tools()
result = await client.call_tool("tool_name", {"arg": "value"})
```

新包名实现：
```python
from sofapy_base.mcp.mcp_manager import get_mcp_manager

mcp_manager = get_mcp_manager()

async with await mcp_manager.get_mcp_client(server_code="remote-mcp") as client:
    await client.connect_to_server()
    tools = await client.list_tools()
    result = await client.call_tool("tool_name", arguments={"arg": "value"})
```

详细说明参见 [MCP Client使用指南](06_mcp_client_usage_guide.md)。

## 八、Web服务变更

### 8.1 Web框架变更（Flask → FastAPI）

旧包名使用 Flask 框架：
```python
from sofapy.core.app import Sofapy

app = Sofapy("my-app")

@app.route("/api/hello", methods=["GET"])
def hello():
    return {"message": "Hello"}

if __name__ == "__main__":
    app.run(host="0.0.0.0", port=8888)
```

新包名使用 FastAPI 框架，文件路径 `servers/web/app.py`：
```python
from fastapi import FastAPI
from ant_baselib.tracer import install_tracer_patches, SofaTracerMiddleware
from sofapy_base.logger.logger import get_logger

logger = get_logger("webserver")

app = FastAPI()
install_tracer_patches()
app.add_middleware(SofaTracerMiddleware)

@app.get("/api/hello")
async def hello():
    logger.info("hello endpoint called")
    return {"message": "Hello"}
```

详细开发说明参见 [Web开发指南](05_web_development_guide.md)。

## 九、日志系统变更

旧包名日志使用方式：
```python
from sofapy.core.logger.logger import LogManager

log_manager = LogManager(level="INFO", log_dir="~/logs/my-app")
logger = log_manager.get_logger("my-module")
logger.info("Info message")
```

新包名日志使用方式：
```python
from sofapy_base.logger.logger import get_logger

logger = get_logger("my-module")
logger.info("Info message")
# 日志自动包含 traceid
```

详细说明参见 [Logger使用指南](07_logger_usage_guide.md)。

## 十、中间件配置变更

| 中间件 | 旧包名配置位置 | 新包名配置位置 |
|--------|--------------|---------------|
| Mist | `middlewares_config.mist` | `module_config.mist` |
| ZCache | `middlewares_config.zcache` | `module_config.zcache` |
| SOFAMQ | `middlewares_config.sofamq` | `module_config.sofamq` |

以 Mist 配置为例，旧包名使用字典配置：
```python
config_dict = {
    "middlewares_config": {
        "mist": {
            "tenant": "ALIPAY",
            "mode": "SHARE"
        }
    }
}
```

新包名使用 YAML 配置：
```yaml
module_config:
  mist:
    tenant: "ALIPAY"
    mode: "SHARE"
```

## 十一、Tracer变更

旧包名 Tracer 使用方式：
```python
from sofapy.core.app import install_tracer_patches

install_tracer_patches(config)
```

新包名 Tracer 使用方式：
```python
from ant_baselib.tracer import install_tracer_patches, SofaTracerMiddleware

install_tracer_patches()
# Web服务需添加中间件
app.add_middleware(SofaTracerMiddleware)
```

详细说明参见 [Tracer使用指南](08_tracer_usage_guide.md)。

## 十二、新包名项目结构

从脚手架 https://sofapy-start.alipay.com/ 获取的标准项目结构：

```
my-app/
├── conf/                       # Docker 配置
│   └── docker/                 # Dockerfile, nginx.conf 等
├── my-app/                     # 应用代码目录（与应用名同名）
│   ├── configs/                # 配置文件目录
│   │   ├── application.yaml    # 主配置
│   │   ├── application-dev.yaml
│   │   ├── application-test.yaml
│   │   └── application-prod.yaml
│   ├── main.py                 # 应用入口（脚手架生成，无需修改）
│   └── servers/                # 服务实现
│       ├── mcp/                # MCP 服务 → 参考 MCP开发指南
│       │   └── app.py
│       ├── sofa/               # SOFA RPC 服务 → 参考 SOFA开发指南
│       │   ├── __init__.py
│       │   ├── app.py
│       │   └── rpc/tr/
│       │           └── hello_service.py
│       └── web/                # Web 服务 → 参考 Web开发指南
│           └── app.py
├── requirements.txt            # Python 依赖
└── README.md
```

启动命令：
```bash
cd my-app
pip install -r requirements.txt
python main.py                    # 默认启动
python main.py -c configs/application-prod.yaml  # 指定配置
```

## 十三、常见问题

### Q1: Flask 路由如何迁移到 FastAPI

| Flask | FastAPI |
|-------|---------|
| `@app.route("/path")` | `@app.get("/path")` |
| `@app.route("/path", methods=["POST"])` | `@app.post("/path")` |
| `request.json` | `request: Model` (Pydantic模型) |
| `jsonify(data)` | 直接返回 `dict` |

### Q2: 如何处理配置中的旧字段

新包名不支持的字段会被忽略，建议清理以下字段：`enable_layotto` 默认已启用，`multi_process` 改用 `workers` 替代，`model`（FAAS/STANDARD）已统一处理。

## 十四、迁移检查清单

### 准备阶段
- [ ] 梳理当前实现（服务类型、中间件使用情况）
- [ ] 从脚手架获取新项目模板 https://sofapy-start.alipay.com/

### 配置迁移
- [ ] 更新 `requirements.txt` 依赖文件
- [ ] 迁移应用配置、中间件配置到 `configs/application.yaml`

### 代码迁移
- [ ] 迁移 SOFA RPC 服务（若有）到 `servers/sofa/`
- [ ] 迁移 MCP 服务（若有）到 `servers/mcp/app.py`
- [ ] 迁移 Web 服务（若有）到 `servers/web/app.py`（Flask → FastAPI）
- [ ] 更新导入路径（`sofapy` → `sofapy_base` / `ant_baselib`）

### 验证阶段
- [ ] 安装依赖 `pip install -r requirements.txt`
- [ ] 本地启动测试 `python main.py`
- [ ] 验证各服务功能正常
- [ ] 更新部署配置

---

如有问题，请联系中间件团队 @文徐 @文喆