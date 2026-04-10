# Tracer 使用指南

## 概述

Tracer 是基于 SofaTracer 的分布式追踪组件，提供调用链追踪、统计监控和自动拦截能力。调用链追踪会自动记录 HTTP/RPC 调用的完整链路，包含 traceId、spanId，便于问题排查。统计监控每分钟汇总调用次数、耗时等指标，用于监控大盘和告警。自动拦截无需修改业务代码，自动拦截 `requests` 和 `SOFARPC` 调用。

## 快速开始

### 基础初始化

在应用启动时调用初始化方法：

```python
from sofapy_base.tracer.tracer import install_tracer_patches

# 最简初始化（应用名从环境变量读取）
install_tracer_patches()

# 或手动指定参数
install_tracer_patches(
    app_name="my-app",
    trace_log_path="~/logs"
)
```

应用名获取的环境变量 `ALIPAY_APP_APPNAME`

## 使用场景

以下所有场景都需要先调用 `install_tracer_patches()` 初始化 tracer，否则无法正常工作。

| 场景 | 注入方式 | 说明 |
| --- | --- | --- |
| HTTP 服务端 | 手动添加 | 中间件需绑定到 FastAPI app 对象，无法自动注入 |
| HTTP 客户端 | 自动拦截 | `requests` 调用自动追踪 |
| RPC 服务端 | 自动拦截 | Layotto RPC 服务处理自动追踪 |
| RPC 客户端 | 自动拦截 | Layotto RPC 调用自动追踪 |

### 1. HTTP 服务端（FastAPI/Starlette）

为 Web 服务添加中间件，追踪入站请求。中间件必须手动添加到 app 对象。

```python
from fastapi import FastAPI
from sofapy_base.tracer.tracer import install_tracer_patches, SofaTracerMiddleware

# 1. 先初始化 tracer（必须在中间件之前）
install_tracer_patches(app_name="my-app")

# 2. 创建应用并手动添加中间件
app = FastAPI()
app.add_middleware(SofaTracerMiddleware, app_name="my-app")

# 中间件会自动：
# - 从请求头提取 trace context
# - 创建服务端 span
# - 记录调用统计
```

### 2. HTTP 客户端调用

使用 `requests` 库发起 HTTP 请求时，自动拦截并记录：

```python
from ant_baselib.tracer import install_tracer_patches

# 1. 先初始化 tracer
install_tracer_patches(app_name="my-app")

# 2. 后续 requests 调用会自动追踪
import requests
response = requests.get("https://example.com/api")
```

如果使用其他 HTTP 库（如 httpx、aiohttp），需要手动记录：

```python
import httpx
import time
from sofapy_base.tracer.tracer.http import StatCollector

start = time.time()
response = httpx.get("https://example.com/api")
elapsed_ms = int((time.time() - start) * 1000)

StatCollector(instance_type='client').record_call(
    url="https://example.com/api",
    method="GET",
    result_code=str(response.status_code),
    stress_test='F',
    response_time_ms=elapsed_ms,
    current_app="my-app"
)
```

### 3. RPC 调用

使用 TR 服务时，自动拦截并记录：

```python
from sofapy_base.tracer.tracer import install_tracer_patches
from layotto import get_mosn_client
from oneapi.basementurl.URLFacadeV2 import URLFacadeV2
from oneapi.basementurl.URLFacadeV2 import com_alipay_basementurl_facade_ShortenRequest

# 1. 先初始化 tracer
install_tracer_patches(app_name="my-app")

# 2. 后续 RPC 调用会自动追踪
layotto_client = get_mosn_client()
service = URLFacadeV2(layotto_client)

req = com_alipay_basementurl_facade_ShortenRequest(
    uid="123123",
    app="your-app-name",
    domain="basementurl.test.alipay.net",
    url="https://example.com"
)
response = service.shorten(req)
```

## 日志说明

### 日志类型

Tracer 输出两类日志，都位于 `~/logs/tracelog/` 目录。追踪日志文件命名为 `*-digest.log`，记录每次调用的详细信息，用于问题排查和调用链可视化。统计日志文件命名为 `*-stat.log`，每分钟汇总调用统计，用于监控大盘和告警。

### 日志文件列表

| 文件名 | 说明 |
| --- | --- |
| `httpclient-digest.log` | HTTP 客户端追踪日志 |
| `httpclient-stat.log` | HTTP 客户端统计日志 |
| `sofa-mvc-digest.log` | HTTP 服务端追踪日志 |
| `sofa-mvc-stat.log` | HTTP 服务端统计日志 |
| `rpc-client-digest.log` | RPC 客户端追踪日志 |
| `rpc-client-stat.log` | RPC 客户端统计日志 |
| `rpc-server-digest.log` | RPC 服务端追踪日志 |
| `rpc-server-stat.log` | RPC 服务端统计日志 |

### 追踪日志格式

以 HTTP 客户端为例，`httpclient-digest.log` 格式：

```
时间,应用名,traceId,rpcId,URL,方法,状态码,请求大小,响应大小,耗时,线程名,目标应用,透传属性...
```

示例：
```
2024-01-01 12:00:00.123,my-app,ac141a171704080000001234,0,https://example.com/api,GET,200,0,1024,150ms,MainThread,target-app,,,
```

### 统计日志格式

以 HTTP 客户端为例，`httpclient-stat.log` 格式：

```
时间,应用名,URL,方法,调用次数,总耗时(ms),结果,压测标记
```

示例：
```
2024-01-01 12:01:00.000,my-app,https://example.com/api,GET,100,15000,Y,F
```

## 高级配置

## Trace Context 透传

调用链追踪依赖 traceId 在服务间传递，Tracer 会自动处理。

HTTP 调用会自动在请求头中注入 `SOFA-TraceId`（链路唯一标识）和 `SOFA-RpcId`（当前节点标识），服务端中间件会自动提取这些头部，保持链路连续。

RPC 调用会自动在 metadata 中注入 `rpc_trace_context.sofatraceid`、`rpc_trace_context.sofarpcid` 和 `rpc_trace_context.sofacallerapp`。

## 常见问题

### 为什么没有生成日志？

确认调用了 `install_tracer_patches()`，确认日志目录有写入权限，确认发生了 HTTP/RPC 调用（追踪日志在 span 结束时输出，统计日志每分钟汇总）。

### 如何验证追踪是否生效？

查看日志文件：

```bash
# 查看追踪日志
cat ~/logs/tracelog/httpclient-digest.log

# 查看统计日志
cat ~/logs/tracelog/httpclient-stat.log
```

## 相关文档

[蚂蚁 Tracer 文档](https://yuque.antfin.com/middleware/tracer)、[HttpClient 日志格式](https://yuque.antfin.com/middleware/tracer/httpclient)、[RPC 日志格式](https://yuque.antfin.com/middleware/tracer/tracer-rpc)