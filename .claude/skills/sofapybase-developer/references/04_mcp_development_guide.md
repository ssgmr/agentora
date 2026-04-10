# MCP 开发指南

本文档介绍如何基于 SOFAPy 框架开发 MCP（Model Context Protocol）服务。MCP 是一种标准化的协议，用于构建 AI 模型与外部工具和数据源的交互接口，支持定义 Tool（工具）和 Prompt（提示词）两类能力。

## MCP 配置示例

```yaml
# configs/application.yaml
app_name: "your_app_name"
enable_sidecar: true
workers: 1                     # MCP 应用只能为 1（即使填其他，框架内部也限定MCP为1）

sidecar_config:
  host: "localhost"
  max_send_message_length: 4194304
  max_receive_message_length: 4194304

log_config:
  trace_log_dir: ''
  log_level: 'INFO'
  log_dir: ''

module_config:
  mcp:
    sub:
      - mcp.ant.{your_app_name}.{your_mcp_server_name}: {}  # value 部分可为空或自定义配置，根据调用方式按需配置
    pub:
      - service_name: "your_mcp_server_name"
        description: "your_mcp_description"
        open_secaspect: true        # 是否开启安全切面，默认 true
        protocol: 0                 # 默认为0，0=SSE, 1=Streamable, 2=Streamable Stateless, 3=SSE+Streamable, 4=SSE+Streamable Stateless
        json_response: false        # 是否返回 JSON 格式响应，默认 false
        # path: "/mcp/"             # # 可选，自定义路径前缀
      - service_name: "your_mcp_server2_name"
        description: "your_mcp2_description"
        open_secaspect: true
        protocol: 0
        json_response: false
        path: "/mcp2/"              # 多 server 时必须配置不同 path
    start: "servers.mcp.app"
```

`sub` 配置调用的 MCP Server 列表（如无需调用主站 MCP Server 可不配置），`pub` 配置发布的 MCP Server，`start` 指定 MCP Server 模块路径（`servers.mcp.app` 对应在 `servers/mcp/app.py` 中定义 MCP Tools 和 Prompts）。请严格遵循以上配置结构，仅配置实际需要的 MCP Server，实际配置值需与用户确认。

## 创建 MCP 工具 (Tool)

使用 `@tool` 装饰器定义工具：

```python
# servers/mcp/app.py
from sofapy_base.mcp.registry import tool

@tool("your_mcp_server_name")
async def add(a1: int, a2: int) -> str:
    """加法计算工具

    Args:
        a1: 第一个加数
        a2: 第二个加数

    Returns:
        加法结果的字符串表示
    """
    value = a1 + a2
    return str(a1) + " + " + str(a2) + " = " + str(value)
```

装饰器参数 `your_mcp_server_name` 需与配置文件中 `pub` 的 `service_name` 一致。工具函数必须是 async 异步函数。服务名拼装规则为 `mcp.ant.{app_name}{.group}.{mcp_server_name}{:uniqId}`，group 和 uniqId 可选，例如 app_name 为 sofapyapp、mcp_server_name 为 sofapyserver 时，最终服务名为 `mcp.ant.sofapyapp.sofapyserver`。

服务发布到 MCP Center 后的参数描述，可通过 `Annotated` 声明：

```python
from typing import Annotated
from pydantic import Field

@tool("sofapyserver")
async def hello(
    int_annotated: Annotated[int, Field(description="参数描述")] = 5,
) -> str:
    return 'hello'
```

## 创建 MCP 提示词 (Prompt)

使用 `@prompt` 装饰器定义提示词：

```python
from mcp.server.fastmcp.prompts import base
from sofapy_base.mcp.registry import prompt

@prompt("your_mcp_server_name")
def review_code(code: str) -> str:
    """代码审查提示词

    Args:
        code: 需要审查的代码

    Returns:
        提示词内容
    """
    return f"Please review this code:\n\n{code}"
```

返回消息列表可用于构建预设的对话上下文：

```python
@prompt("sofapyserver")
def debug_error(error: str) -> list[base.Message]:
    """错误调试提示词

    Args:
        error: 错误信息

    Returns:
        消息列表
    """
    return [
        base.UserMessage("I'm seeing this error:"),
        base.UserMessage(error),
        base.AssistantMessage("I'll help debug that. What have you tried so far?"),
    ]
```

## 获取请求上下文信息

通过 `Context` 参数获取请求上下文：

```python
from antmcp.server.fastmcp import Context
from antmcp.utils.tracer import get_rpc_id, get_trace_id

@tool("your_mcp_server_name")
async def get_context_info(ctx: Context) -> str:
    """获取请求上下文信息

    Args:
        ctx: 请求上下文

    Returns:
        响应消息
    """
    headers = ctx.request_context.request.headers
    session_id = ctx.request_context.request.query_params.get("session_id")
    rpc_id = get_rpc_id()
    trace_id = get_trace_id()

    logger.info(f"rpc_id={rpc_id}, trace_id={trace_id}, session_id={session_id}")
    return f"rpc_id={rpc_id}, trace_id={trace_id}, session_id={session_id}"
```

可获取 `ctx.request_context.request.headers`（HTTP 请求头）、`ctx.request_context.request.query_params`（查询参数）、`get_rpc_id()`（RPC 追踪 ID）、`get_trace_id()`（链路追踪 ID）。详细的 Tracer 配置与使用方式详见 [Tracer 使用指南](08_tracer_usage_guide.md)。

## 在 MCP 应用中调用 SOFARPC(TR) 服务

配置及调用方式详见 [TR 调用指南](11_tr_usage_guide.md)。

## 使用蚂蚁中间件能力

支持 DRM、ZDAS、ZCache、DDSOSS、SOFAMQ、SOFAMQX、Msgbroker 等中间件，配置及使用方式详见 [中间件使用指南](10_middleware_usage_guide.md)。

## 验证方法

完成 MCP 服务开发后，执行以下步骤验证功能是否正常。

### 1. 启动服务

在项目根目录（与 requirements.txt 同级）执行：

```bash
source .venv/bin/activate   # 激活虚拟环境
cd <应用名>                  # 进入应用目录（如 cd sofapyapp）
python main.py              # 启动服务，后台运行可添加 2>&1 &（需通过 ps aux | grep main.py 查找进程后 kill <pid> 终止）
```

等待日志输出 `Application startup complete.` 表示服务启动成功。

### 2. 使用 MCP Client 调用验证

通过 `mcp_manager` 调用本地 MCP 服务进行验证：

```python
from sofapy_base.mcp.mcp_manager import get_mcp_manager

mcp_manager = get_mcp_manager()

async with await mcp_manager.get_mcp_client(
    server_code='mcp.ant.{app_name}.{server_name}'
) as client:
    await client.connect_to_server()
    tools = await client.list_tools()
    result = await client.call_tool("tool_name", arguments={...})
```

详细的 MCP Client 配置与调用方式详见 [MCP Client 使用指南](06_mcp_client_usage_guide.md)。

### 3. 检查结果

确认 `list_tools` 返回的 tool 列表中包含你定义的工具，确认 `call_tool` 返回结果符合预期，检查服务日志中是否有异常信息。

### 4. 常见问题排查

| 问题 | 可能原因 | 解决方案 |
| --- | --- | --- |
| 连接失败 | 服务未启动或 server_code 错误 | 检查服务状态和 server_code 格式 |
| Tool 不存在 | tool 未正确注册 | 检查 `@tool` 装饰器的 service_name 与配置是否一致 |
| 调用超时 | 业务逻辑执行时间过长 | 检查 tool 实现是否有阻塞操作 |