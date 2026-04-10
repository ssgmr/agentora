# 常见问题汇总

本文档汇总了 SOFAPy 框架开发过程中的常见问题及解决方案。

本文档涉及以下需要用户手动完成的操作，Agent 无法完成：登陆线上或测试机器执行命令（如健康检查、查看日志、重启服务等）、修改系统配置（如 /etc/hosts）、访问内部平台（如语雀文档、Spanner、MVN 上传等）、申请资源（如域名、中间件实例等）。遇到此类问题时，请引导用户提供必要信息或指导其自行操作。

---

## 配置与初始化

### 如何配置用户自定义配置项

在 `application.yaml` 中添加 `user_config` 字段即可实现自定义配置。配置示例：

```yaml
# 基础配置
app_name: "sofapyapp"
enable_sidecar: true
workers: 1

# 用户自定义配置 - 可以在这里添加任何自定义字段
user_config:
  app:
    title: "My SOFAPy App"
    version: "1.0.0"
    description: "Custom SOFAPy application"

  features:
    enable_cache: true
    enable_metrics: false
    max_connections: 100

  business:
    timeout: 30
    retry_count: 3
```

代码中通过以下方式获取自定义配置：

```python
from sofapy_base.app.config import get_config

config = get_config()
user_config = config.user_config
```

---

## TR 相关

### 如何指定 TR 服务目标 IP 地址与请求超时时间

设置目标 IP 地址仅限本地调试使用，线上环境不可使用。在调用 TR 服务时，通过 `metadata` 传入指定目标 IP 和端口：

```python
try:
    resp = HelloFacade2.echo(
        EchoRequest(message2),
        layotto_options={
            "metadata": {"rpc_target_address": "127.0.0.1:12200"},
        },
    )
    logger.debug(resp.data)
    logger.info(resp.get_json())
```

设置请求超时时间通过 `rpc_request_timeout` 参数指定请求超时阈值，该参数为字符串类型，单位为毫秒：

```python
try:
    resp = facade.trigger_full_update(
        layotto_options={
            "metadata": {"rpc_request_timeout": "3000"},
        },
    )
```

### rpc.proto 中 repeated 生成的 java 类型非 List 怎么办

首先删除老版本的 oneapi-codegen，然后通过 tnpm 安装新版本：

```bash
# 先找到目录地址, 再删除
rm -rf $(which oneapi-codegen)

# 安装新版本
tnpm i -g @alipay/oneapi-codegen-sdk
```

通过下面命令查看版本，确保版本 >= 5.10.0：

```bash
oneapi-codegen --version
# 输出示例: 5.10.1
```

---

## MCP 相关

MCP Client 使用相关问题（超时设置、tracer 透传、agent_id 配置等）详见 MCP Client 使用指南文档。

### 办公网链路 MCP 服务端可以获取调用者身份信息吗

参考文档：[办公网MCP链路中身份信息传递指南](https://yuque.antfin.com/middleware/sofagw/svm241gc2iokbml3)

### sofapy 框架怎么把 TR 服务的 tracer 信息塞入上下文

```python
@app.rpc(
    service_name='com.alipay.layotto.layotto_oneapi_pb_testapp.HelloFacade',
    method='echo',
)
def echo_handler(request: RpcRequest) -> SofaRpcResponse:
    echoRequest = EchoRequest(**request.data)
    trace_id = request.metadata.get('rpc_trace_context.sofatraceid')
    span_id = request.metadata.get('rpc_trace_context.sofarpcid')

    from antmcp.utils.tracer import set_tracer
    set_tracer(trace_id, span_id)
```

### sofapy 框架 sofamq 发送消息指定 tracer 信息

首先需要升级 layotto 的版本依赖：

```bash
layotto = ">= 2026.01.29.1"
```

代码示例：

```python
from sofapy_base.app.layotto_manager import get_layotto_manager
from antmcp.utils.tracer import get_rpc_id, get_trace_id
from layotto.core.consts import RPC_TRACE_CONTEXT_SOFA_RPC_ID, RPC_TRACE_CONTEXT_SOFA_TRACE_ID

manager = get_layotto_manager()


# 使用装饰器注册消息发布器
@manager.sofamq_manager.publisher(
    topic="TP_ISEECELERY_KOMBU_TEST",
    group="GID_ISEECELERY_KOMBU_TEST",
    tag="python",  # 消息标签（可选）
    properties={"test_property": "test_value"}  # 自定义消息属性（可选）
)
def publish_to_sofamq(message_ str | bytes):  # 自定义生产者函数名
    """
    将 message_data 作为消息体发送到 SOFAMQ。
    该函数可多次调用，每次都会立即生成一条新消息并投递到 SOFAMQ
    """
    return message_data


# 发送消息时透传 tracer 信息
publish_to_sofamq(
    "Hello, I am SOFAMQ",
    properties={
        RPC_TRACE_CONTEXT_SOFA_TRACE_ID: get_trace_id(),
        RPC_TRACE_CONTEXT_SOFA_RPC_ID: get_rpc_id()
    }
)
```

---

## 错误排查

### nodename nor servname provided, or not known

原因是主机名解析失败。解决方案需在目标机器手动执行，运行以下命令将 host 添加到 /etc/hosts 中：

```bash
sudo sh -c 'echo "127.0.0.1 $(hostname)" >> /etc/hosts'

# 重启一下 meshboot，命令内替换为项目的应用名
meshboot restart -m binary -a {your app name}
```

### dds-oss: choose oss client, MESSAGE: Not found: no read oss

错误信息：

```
grpc._channel._MultiThreadedRendezvous: <_MultiThreadedRendezvous of RPC that terminated with:
        status = StatusCode.INTERNAL
        details = "get file fail,err: rpc error: code = NotFound desc = DDSOSS OP: choose oss client, MESSAGE: Not found: no read oss"
        debug_error_string = "UNKNOWN:Error received from peer  {grpc_message:\"get file fail,err: rpc error: code = NotFound desc = DDSOSS OP: choose oss client, MESSAGE: Not found: no read oss\", grpc_status:13}"
```

解决方案需用户手动操作：带 mesh 重启下机器，新增或修改 dbkey 后要重启 mosn。

### 404 Client Error: Not Found for url: http://127.0.0.1:13330/mcp/publish

原因是 Mosn 版本太低，解决方案是升级到最新版本即可。

---

## 运维与部署

### 线上线下 Mosn 接入和升级

需用户参照文档手动操作。操作手册：https://yuque.antfin.com/sofa-open/cnar/rk8fwq#D3VmD，Mosn 版本信息：https://yuque.antfin.com/mesh/service-mesh/mosn-release-note-latest

### 在应用内获取当前运行环境

```python
from antmcp.utils import get_env

env = get_env()

# 返回值说明：
# prod   - 线上
# gray   - 灰度
# prepub - 预发
# test   - 测试
# stable - stable
# dev    - 开发
```

### 如何修改构建脚本中的 Python 版本

在 Dockerfile 中修改对应的 Python 版本：

```dockerfile
FROM reg.docker.alibaba-inc.com/aci-images/python-service:3.12.0-564036949

# init folder
RUN mkdir -p /home/admin/logs/sofapyapp && mkdir -p /home/admin/logs/.logrotate && mkdir -p /home/admin/bin && mkdir -p /home/admin/conf

# init env and install software
RUN yum install -y tengine-proxy-2.5.12 -b current
RUN yum install -y cronolog-1.6.2 -b current

# install requirements.txt
COPY --chown=admin:admin requirements.txt /home/admin/release/
RUN python3.12 -m venv /home/admin/run && \
    . /home/admin/run/bin/activate && \
    python3.12 -m pip install -i https://pypi.antfin-inc.com/simple \
    --extra-index-url https://artifacts.antgroup-inc.cn/simple/ \
    --extra-index-url https://pypi.antfin-inc.com/artifact/repositories/simple-dev/ \
    -r /home/admin/release/requirements.txt

# copy source file
COPY --chown=admin:admin sofapyapp /home/admin/release/sofapyapp

# copy scripts
COPY --chown=admin:admin conf/docker/scripts/admin /home/admin
RUN chmod +x /home/admin/bin/*.sh

COPY --chown=admin:admin conf/docker/nginx.conf /home/admin/release/
COPY --chown=admin:admin conf/docker/logrotate.conf /home/admin/release/

RUN chown admin:admin -R /home/admin
```

基础镜像地址：https://yuque.antfin.com/linkex/help/efvmd535xmv7ltig

### SOFAPy 框架应用状态健康检查

需登陆目标机器执行：

```bash
cd /home/admin/bin/
sh health.sh
```

返回示例：

```json
{
  "status": "healthy",
  "web_health": "healthy",
  "mcp_health": "healthy",
  "sofa_health": "healthy",
  "timestamp": 1773750795.3422,
  "processes": {
    "Middleware-Worker-0": {
      "pid": 10140,
      "alive": true,
      "exitcode": null
    },
    "Web-Worker": {
      "pid": 10141,
      "alive": true,
      "exitcode": null
    },
    "MCP-Worker": {
      "pid": 10142,
      "alive": true,
      "exitcode": null
    }
  }
}
```

### SOFAPy 框架启动失败怎么办

机器登陆不了时，首先确保 start.sh 里面包含日志目录的创建：

```bash
#!/bin/bash
######################################################################
# start.sh
# 应用业务进程启动脚本
# 需要在此脚本中拉起业务需要的所有进程
#
# 注意：脚本返回非0时，容器启动会失败。
######################################################################

mkdir -p /home/admin/logs/sofapyapp

# 启动 LOGROTATE 服务
/home/admin/bin/start-logrotate.sh

# 初始化 MOSN
/home/admin/bin/init-mosn.sh

# 激活虚拟环境
. /home/admin/run/bin/activate

# 启动应用
cd /home/admin/release/sofapyapp
nohup python main.py >> /home/admin/logs/sofapyapp/start.log 2>&1 &

# 启动 Tengine
/opt/taobao/tengine/bin/tengine -c /home/admin/release/nginx.conf

exit 0
```

如果应用启动失败，可以查看日志（需登陆目标机器）：`/home/admin/logs/sofapyapp/start.log`

---

## 其他

### Mosn 值班地址

[Mosn 小蜜Links](https://links.alipay.com/app/room/5e9d93a2dcf940ab86f942a9/)

### MCP 网关接入手册

https://yuque.antfin.com/middleware/sofagw/rg0t9bioy70usxtm#e38lc

### 怎么申请 Web 域名

需用户自行在 Spanner 平台申请，通过 [域名接入 Spanner](https://paas.alipay.com/jiuzhou-portal/dashboard?appListSettings=false&changeCategorySelection=spanner-operator) 完成。