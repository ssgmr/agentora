# MCP Client 使用指南

本文档介绍如何使用 MCP Client 调用远程 MCP Server 服务。SOFAPy 的 MCP Client 对接了蚂蚁的注册中心，支持非网关模式（有 Mosn 环境，通过注册中心服务发现）和网关模式（办公网/无 Mosn 环境，直接指定网关地址）两种调用方式。

## 一、非网关模式（调用 Client 所在环境的 MCP Server）

该方式通过注册中心进行服务发现，请确保目标服务在对应环境已部署。

### 配置

```yaml
# 基础配置
app_name: "sofapyapp"
enable_sidecar: true
workers: 1

# Sidecar 配置
sidecar_config:
  host: "localhost"
  max_send_message_length: 4194304     # 4MB
  max_receive_message_length: 4194304  

log_config:
  trace_log_dir: ''
  log_level: 'INFO'
  log_dir: ''

# 中间件配置
module_config:
  mcp:
    sub:
      - mcp.ant.sofadoc.sofadocmcpserver: {}
      - mcp.ant.faas.deriskMcpServerGroup.mcpAntscheduler: {"mesh_vip_address": "faasgw-pool:8080"}  # 通过 faas 模式
```

`module_config.mcp.sub` 配置需要调用的 MCP 服务列表，格式为 `服务名: {配置参数}`，服务名格式为 `mcp.ant.{app_name}.{mcp_server_name}`，可选参数（如 `mesh_vip_address`）根据目标服务要求配置。

### 代码调用

**方式一：通过 mcp_manager 调用（推荐）**

MCP 的能力封装在 MCPManager 中，适用于 SOFAPy 应用内调用，自动管理连接生命周期。

```python
import asyncio

from sofapy_base.mcp.mcp_manager import get_mcp_manager
from sofapy_base.logger.logger import get_logger

logger = get_logger("mcpclient")

# 获取全局 MCP Manager 实例
mcp_manager = get_mcp_manager()

async def call_mcp_service():
    """调用 MCP 服务"""
    try:
        # 获取 MCP 客户端（自动清理资源）
        async with await mcp_manager.get_mcp_client(
            server_code='mcp.ant.sofapyapp.sofapyserver'
        ) as client:
            # 建立连接
            await client.connect_to_server()

            # 获取可用工具列表
            tools = await client.list_tools()
            logger.info("Available tools: %s", tools)

            # 调用工具
            result = await client.call_tool(
                "tool_name",
                arguments={"param1": "value1"},
                timeout=10
            )
            logger.info("Tool result: %s", result)

    except Exception as e:
        logger.error("MCP call failed: %s", e)

if __name__ == "__main__":
    asyncio.run(call_mcp_service())
```

**方式二：直接使用 MCPClient**

适用于独立脚本或需要更多控制的场景：

```python
from antmcp.client.client import MCPClient

# 通过此方法创建 client 默认走 SSE 协议
client = MCPClient.create(service_code)

# 连接并调用
await client.connect_to_server()
tools = await client.list_tools()
result = await client.call_tool("tool_name", arguments={"key": "value"})
```

### 线上线下环境 Mosn 接入与升级

非 Java 语言接入 Mosn 必须指定版本，详见 [Mosn 发布说明](https://yuque.antfin.com/sofa-open/cnar/rk8fwq#D3VmD)。线上线下环境接入和升级详见 [Mosn 接入手册](https://yuque.antfin.com/sofa-open/cnar/rk8fwq#D3VmD)。

## 二、网关模式（办公网调用线上环境服务/无法安装 Mosn 的环境）

该方式通过网关访问 MCP Server，适用于办公网调用预发/线上服务、或无法安装 Mosn 的环境。

### 代码示例

访问固定 IP/自定义域名/MCP Center 上的服务：

```python
import asyncio

from sofapy_base.logger.logger import get_logger
from antmcp import MCPClient
from sofapy_base.tracer.tracer import install_tracer_patches

logger = get_logger("mcpclient")

# 安装 tracer 插件
install_tracer_patches()

async def test_mcpnexus():
    """通过网关调用 MCP 服务"""

    service_code = "mcp.ant.sofapyapp.sofapyserver"
    client = None

    try:
        # 创建 MCP 客户端，指定网关地址
        client = await MCPClient.create(
            service_code,
            host="https://mcpnexus-prod.alipay.com"
        )

        # 建立连接（可传入认证信息）
        await client.connect_to_server(
            properties={
                "Authorization": "your_I_AM_token"
            }
        )

        # 获取工具列表
        tools = await client.list_tools()
        logger.info("Available tools: %s", tools)

        # 发送心跳
        await client.send_ping()

        # 调用工具
        resp = await client.call_tool(
            "get_shorturl",
            timeout=10
        )
        logger.info("get_shorturl Response: %s", resp)

        resp = await client.call_tool(
            "add",
            arguments={"a1": 100, "a2": 200},
            timeout=10
        )
        logger.info("add Response: %s", resp)

    except Exception as e:
        logger.error("Error while calling tool: %s", e)
    finally:
        # 确保资源清理
        if client:
            await client.cleanup()

if __name__ == "__main__":
    asyncio.run(test_mcpnexus())
```

## 三、高级配置

### 指定 agent_id/应用名

如需自定义身份信息，可在创建 client 时指定：

```python
from antmcp.client.client import MCPClient

client = MCPClient.create("your_service_code", agent_id="your_agent_id/app_name")
```

### 设置超时时间

全局配置：

```python
from antmcp.client.client import set_global_read_timeout

# 配置全局超时时间 10s
set_global_read_timeout(10)
```

单次请求配置：

```python
client.call_tool('tool_name', timeout=10)
```

### 启用 Tracer 能力

```python
from antmcp.client.client import MCPClient
from sofapy_base.tracer.tracer import install_tracer_patches

# 全局安装 tracer
install_tracer_patches()

# 创建 client
client = MCPClient.create(service_code)
```

### 透传 trace_id 和 span_id

基于 SOFAPy 开发的服务端调用 MCP Client 会自动劫持和注入 tracer。若独立使用 MCP Client，需手动注入：

```python
from antmcp.client.util import set_tracer

# 替换成自己的 tracer 信息
set_tracer("trace_id", "span_id")
```

### 设置自定义 x-one-id

SOFAPy 默认自动生成 x-one-id，如需自定义，可卸载 mist 的 hook：

```python
from antmcp.client.hooks import unregister_request_hook, inject_mist_method

unregister_request_hook(inject_mist_method)
```

## 四、常见问题

### 如何查看 MCP Server 的 service_code？

登陆 antshell 机器执行命令（service_name 为 yaml 配置中的 service_name）：

```bash
# 请替换 sofapyserver 为你的 service_name
grep -r "sofapyserver" ~/logs/mosn/endpoint.access.log*
```

### 如何查看 MCP Server 发布的 Tools？

连接成功后，调用 `list_tools()` 方法：

```python
tools = await client.list_tools()
print(tools)
```

### 常见错误排查

| 问题 | 可能原因 | 解决方案 |
| --- | --- | --- |
| 连接失败 | 服务未启动或 server_code 错误 | 检查服务状态和 server_code 格式 |
| Tool 不存在 | tool 未正确注册 | 检查目标 MCP Server 的 tool 定义 |
| 调用超时 | 业务逻辑执行时间过长 | 调整超时时间或优化业务逻辑 |
| 404 Client Error: Not Found for url | Mosn 版本太低 | 升级 Mosn 到最新版本 |

更多问题详见 [常见问题汇总](20_faq_guide.md) 的 MCP 相关章节。