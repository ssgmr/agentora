# SOFAPy 应用日志排查指南

文档中的 `<应用名>` 需替换为实际应用名，即 `application.yaml` 中 `app_name` 的值。

## 日志目录位置

本地开发环境日志根目录默认为 `~/logs/`，生产环境默认为 `/home/admin/logs/`。

目录结构：
```
~/logs/
├── <应用名>/                    # 应用业务日志
│   ├── <模块名>.log            # 各模块主日志
│   ├── <模块名>-error.log      # 模块 ERROR 日志
│   ├── <模块名>-fatal.log      # 模块 CRITICAL 日志
│   └── common-error.log        # 汇总错误日志（推荐优先查看）
└── tracelog/                    # 链路追踪日志
    ├── rpc-client-digest.log   # RPC 调用摘要
    ├── ai-mcp-*-digest.log     # MCP 调用摘要
    ├── httpclient-digest.log   # HTTP 调用摘要
    └── sofa_trace_self.log     # Tracer 自身日志（包含错误栈）
```

应用日志文件名由 `get_logger("模块名")` 调用时传入的参数决定，如传入 `"webserver"` 则生成 `webserver.log`。

## 日志格式

应用日志格式：`时间戳 - 模块名 - 日志级别 - [进程ID] - [进程名] - [TraceId] - 日志内容`

示例：
```
2026-03-19 13:18:50,507 - <模块名> - INFO - [42445] - [MainProcess] - [-] - 应用开始启动
2026-03-19 13:20:32,552 - <模块名> - ERROR - [42452] - [SpawnProcess-2:3] - [7f0000011773897631811100542452] - Error while calling tool
```

TraceId 字段用于链路追踪，`-` 表示无 TraceId。

## TraceId 链路追踪

TraceId 为 32 位十六进制字符串（如 `7f0000011773897631811100542452`），可关联一个请求的完整调用链路。

在 RPC 服务中获取：
```python
def handle_request(request):
    trace_id = request.metadata.get('rpc_trace_context.sofatraceid')
```

在 MCP 服务中获取：
```python
from antmcp.utils.tracer import get_trace_id

def handle_request():
    trace_id = get_trace_id()
```

通过 TraceId 追踪请求：
```bash
TRACE_ID="7f0000011773897631811100542452"
grep "$TRACE_ID" ~/logs/<应用名>/*.log
grep "$TRACE_ID" ~/logs/tracelog/*.log
```

## 常见问题排查

### 应用启动失败

查看框架启动日志：
```bash
# 查看模块日志
head -50 ~/logs/<应用名>/<模块名>.log
# 容器环境
tail -100 /home/admin/logs/<应用名>/start.log
```

正常启动日志：
```
2026-03-19 13:18:50,507 - <模块名> - INFO - [...] - SOFAPy应用开始启动
2026-03-19 13:18:50,513 - <模块名> - INFO - [...] - Middleware Worker 0 started (PID: 42447)
```

### 业务错误

查看错误日志：
```bash
# 查看汇总错误
tail -100 ~/logs/<应用名>/common-error.log

# 查看具体模块错误
tail -100 ~/logs/<应用名>/<模块名>-error.log

# 根据 TraceId 追踪
grep "YOUR_TRACE_ID" ~/logs/<应用名>/*.log
```

### 网络连接错误

网络错误通常记录在 `sofa_trace_self.log` 中，包含完整错误栈：
```bash
tail -100 ~/logs/tracelog/sofa_trace_self.log
```

### RPC 调用问题

```bash
# 查看模块日志
tail -100 ~/logs/<应用名>/<模块名>.log

# 查看调用摘要
tail -100 ~/logs/tracelog/rpc-client-digest.log
```

## 常用命令

实时监控错误日志：
```bash
tail -f ~/logs/<应用名>/common-error.log
```

查看所有 ERROR 日志：
```bash
grep -h "ERROR" ~/logs/<应用名>/*.log | tail -50
```

按时间范围查看：
```bash
sed -n '/2026-03-19 13:18:50/,/2026-03-19 13:20:00/p' ~/logs/<应用名>/<模块名>.log
```

查看日志大小：
```bash
du -sh ~/logs/*
du -h ~/logs/<应用名>/* | sort -hr
```

