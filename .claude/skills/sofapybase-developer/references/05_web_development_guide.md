# Web 开发指南

本文档介绍如何基于 SOFAPy 框架进行配置和开发 Web 服务。

## Web 配置示例

```yaml
app_name: "your_app_name"
enable_sidecar: true
workers: 1      # 指定启动的进程数，当前只对 Web 应用生效，相当于 uvicorn 的 workers

sidecar_config:
  host: "localhost"
  max_send_message_length: 4194304
  max_receive_message_length: 4194304

log_config:
  trace_log_dir: ''
  log_level: 'INFO'
  log_dir: ''

module_config:
  web:
    port: 8888    # 如修改端口，需同步修改 conf/docker/nginx.conf
    start: "servers.web.app:app"    # Web 服务启动入口，默认为"servers.web.app:app"，格式为 {模块路径}:{实例名}，对应 servers/web/app.py 中的 app 实例
```

`servers.web.app:app` 对应 FastAPI 应用实例。如果用户修改了目录结构，请调整对应的 start 配置。

## 创建路由

默认采用 FastAPI 框架，支持替换为其他 ASGI 兼容框架。

### 基础路由

在 `servers/web/app.py` 中添加路由：

```python
from fastapi import FastAPI
from typing import Dict

app = FastAPI()

@app.get("/hello")
async def hello() -> Dict[str, str]:
    """Hello 接口示例"""
    return {"message": "hello, i am sofapy"}
```

如需启用 tracer 日志追踪：

```python
from ant_baselib.tracer import (
    SofaTracerMiddleware,
    install_tracer_patches
)

# 安装 tracer 插件
install_tracer_patches()
# 添加 tracer 中间件
app.add_middleware(SofaTracerMiddleware)
```

详细的 Tracer 配置与使用方式详见 [Tracer 使用指南](08_tracer_usage_guide.md)。

### 使用 APIRouter 组织路由

项目规模增大时，建议使用 APIRouter 拆分路由到不同文件：

```python
# routers/user.py
from fastapi import APIRouter

router = APIRouter(prefix="/users", tags=["users"])

@router.get("/")
async def list_users():
    return [{"id": 1, "name": "user1"}]

# app.py
from fastapi import FastAPI
from routers.user import router as user_router

app = FastAPI()
app.include_router(user_router)
```

## 在 Web 应用中调用 MCP Server

前置条件：需先在 `application.yaml` 中配置要调用的 MCP Server：

```yaml
module_config:
  mcp:
    sub:
      - mcp.ant.sofadoc.sofadocmcpserver: {}
```

```python
from sofapy_base.mcp.mcp_manager import get_mcp_manager

mcp_manager = get_mcp_manager()

@app.get("/sofadoc")
async def sofadoc():
    """调用远程 MCP 服务"""
    async with await mcp_manager.get_mcp_client(
        server_code='mcp.ant.sofadoc.sofadocmcpserver'
    ) as client:
        await client.connect_to_server()
        tools = await client.list_tools()
        result = await client.call_tool("tool_name", arguments={"key": "value"})
        return result
```

详细的 MCP Client 配置与调用方式详见 [MCP Client 使用指南](06_mcp_client_usage_guide.md)。

## 在 Web 应用中调用 SOFARPC(TR) 服务

配置及调用方式详见 [TR 调用指南](11_tr_usage_guide.md)。

## 在 Web 应用中使用中间件（以 DRM 为例）

前置条件：需先在 `application.yaml` 中启用中间件：

```yaml
module_config:
  drm:
    enabled: true
```

```python
from servers.web.layotto_manager import manager

# 获取 DRM 配置，get_drm_config 方法返回 DRMConfiguration 对象（来自 layotto）
config = manager.drm_manager.get_drm_config("config_id")
print(f"Key: {config.key}")
print(f"Value: {config.value}")
print(f"Version: {config.version}")
```

其他中间件使用方式详见 [中间件使用指南](10_middleware_usage_guide.md)。

## 验证

完成 Web 接口开发后，执行以下步骤验证功能是否正常。

启动服务时，在项目根目录（与 requirements.txt 同级）执行以下命令：

```bash
source .venv/bin/activate   # 激活虚拟环境
cd <应用名>                  # 进入应用目录（如 cd sofapyapp）
python main.py              # 启动服务，后台运行可添加 2>&1 &
```

等待日志输出 `Application startup complete.` 表示服务启动成功。

调用接口验证时，使用 curl 或其他 HTTP 客户端调用接口：

```bash
curl http://localhost:8888/hello    # 替换为实际api
```

检查响应时，确认 HTTP 状态码为 200，确认返回数据结构符合预期，并检查日志中是否有异常信息。