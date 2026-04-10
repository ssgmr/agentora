# SOFARPC 开发指南

本文档介绍如何基于 SOFAPy 框架开发 TR 服务。TR 协议是蚂蚁内部的 RPC 通信协议，用于服务间的调用。

## 研发准备

### 1. 安装 oneapi-codegen

检查是否已安装：

```bash
oneapi-codegen -v
```

若未安装，详见 [oneapi-codegen 安装指南](09_oneapi_codegen_install_guide.md)。

### 2. 编写 rpc.proto 定义服务接口

进入应用包目录（项目根目录下的 `{应用名}` 子目录）：

```bash
cd {应用名}
```

编辑 `rpc.proto` 文件。若文件不存在，请创建：

```bash
touch rpc.proto
```

然后编辑该文件，填入服务接口定义内容。

#### Protobuf 服务定义规范示例

```protobuf
// 语法版本：固定为 oneapi（蚂蚁内部基于 proto3 的扩展语法），勿修改
syntax = "oneapi";

// 接口描述版本号（主版本.次版本.修订版本），默认 1.0.0
option version = "1.0.0";

// package 格式：com.alipay.layotto.{应用名}
// 示例：应用名为 sofapyapp，则写 com.alipay.layotto.sofapyapp
package com.alipay.layotto.sofapyapp;

// ============================================================
// Message 定义（数据结构）
// 命名规范：{方法名}Request / {方法名}Response，大驼峰
// 字段命名：下划线风格
// ============================================================

// 请求体结构：字段名和类型按实际业务需求定义

message SayHelloRequest {
  string name = 1;  // 字段编号一旦设定不应更改，后续新增字段使用递增编号
}

// 响应体结构：字段名和类型按实际业务需求定义
message SayHelloResponse {
  string message = 1;
  bool success = 2;
}

// ============================================================
// Service 定义
// 命名规范：{Facade名}，大驼峰
// 方法命名：小驼峰
// ============================================================

service HelloFacade {
  // 方法命名规范：小驼峰命名
  rpc sayHello(SayHelloRequest) returns (SayHelloResponse);
}

// ============================================================
// 【附录】Protobuf 支持的常用字段类型：
//   string    - 字符串
//   int32     - 32位整数
//   int64     - 64位整数
//   bool      - 布尔值
//   float     - 32位浮点数
//   double    - 64位浮点数
//   bytes     - 字节数组
//   repeated  - 列表/数组（如 repeated string tags = 1;）
//   嵌套 message 或 enum
// ============================================================
```

### 3. 生成接口代码

#### 执行前环境检查

在执行代码生成命令前需确认 Java 环境和 NODE_PATH 环境变量已就绪：

```bash
java -version
echo $NODE_PATH
```

若任一环境缺失，参考 [oneapi_codegen 常见问题](09_oneapi_codegen_install_guide.md#常见问题：环境依赖缺失) 完成配置。

SOFARPC 服务接口代码必须通过 oneapi-codegen 工具生成，不可自行编写。

#### 执行代码生成命令

环境就绪后，进入 `rpc.proto` 所在目录（项目根目录下的 `{应用名}` 子目录）：

```bash
cd {应用名}
```

然后执行代码生成命令（支持指定分支）：

```bash
oneapi-codegen layotto publish --lang=python --branch=master
```

该命令会生成 Java 接口代码（`oneapi/java-home/src`）、构建 JAR 包（`oneapi/java-home/target/{appname}-facade-{version}.jar` 和 `-source.jar`）、以及 Python 代码（`oneapi/{appname}/{facade_name}.py`）。

### 4. 发布 JAR 包（需用户操作）

当 Python 应用需要向 SOFA（Java 体系）应用提供 TR 服务时，需将 JAR 包上传至 Maven 仓库，确保双方接口协议一致。此步骤涉及内部系统权限，需用户手动操作。

**MVN 线下开发环境**：访问 https://artifacts-web.antgroup-inc.cn/upload/maven ，选择「制品上传」-「Maven2」，「目标仓库」选择"Alipay-Releases-dev 开发库，用于常规业务发 JAR"，选择「上传 POM（GAV 从 POM 中读取）」后上传 POM 文件（`oneapi/java-home/pom.xml`）、服务接口 JAR 和源码 JAR，最后选择「确认并上传至制品库」。

**MVN 线上环境**：通过 [LinkE新建无源码三方JAR变更](https://linke.alipay.com/#/alipay/process/create/tpJar?site=MAIN_SITE&tenant=alipay) 上传 JAR 包。

## 实现并发布 TR 服务

### 1. 配置 application.yaml

```yaml
# configs/application.yaml
app_name: "your_app_name"
enable_sidecar: true
workers: 1

sidecar_config:
  host: "localhost"
  max_send_message_length: 4194304
  max_receive_message_length: 4194304

log_config:
  trace_log_dir: ''
  log_level: 'INFO'
  log_dir: ''

module_config:
  sofa:
    start: "servers.sofa.app:app"    # SOFA 服务启动入口，默认为"servers.sofa.app:app"，格式为 {模块路径}:{实例名}，对应 servers/sofa/app.py 中的 app 实例
```

### SOFA 应用实例

框架默认在 `servers/sofa/app.py` 中创建应用实例：

```python
# servers/sofa/app.py
from sofapy_base.app.application import SOFAPyApplication
from ant_baselib.tracer import install_tracer_patches

# 创建应用实例
app = SOFAPyApplication()

# 安装链路追踪补丁（可选，建议保留）
install_tracer_patches()
```

详细的 Tracer 配置与使用方式详见 [Tracer 使用指南](08_tracer_usage_guide.md)。

### 编写服务端接口 handler

使用装饰器模式将业务 handler 注册为 TR 服务端接口。文件位置在 `servers/sofa/rpc/tr/` 目录下，文件名使用 facade name 的 snake_case 形式（如 `HelloFacade` → `hello_facade.py`）。注意导入 `RpcRequest`、`SofaRpcResponse`（来自 `layotto`）。

```python
# servers/sofa/rpc/tr/hello_facade.py
from layotto import RpcRequest, SofaRpcResponse
from sofapy_base.logger.logger import get_logger
from servers.sofa.app import app

logger = get_logger("tr")

@app.rpc(
    service_name='com.alipay.layotto.{your_appname}.{your_facade_name}}',  # 例如 'com.alipay.layotto.sofapyapp.HelloFacade'
    method='sayHello',
)
def say_hello_handler(request: RpcRequest) -> SofaRpcResponse:
    """
    处理 TR 请求

    Args:
        request: RpcRequest 包含 data 和 metadata

    Returns:
        SofaRpcResponse: 返回响应对象
    """
    # 获取请求参数
    hello_request = HelloRequest(**request.data)

    # 获取链路追踪信息
    trace_id = request.metadata.get('rpc_trace_context.sofatraceid')
    span_id = request.metadata.get('rpc_trace_context.sofarpcid')

    logger.info('receive rpc request: %s', hello_request)

    # 业务逻辑处理
    message = f"Hello, {hello_request.name}!"

    # 构造响应
    response = HelloResponse(message=message, success=True)

    return SofaRpcResponse(is_error=False, app_response=response)

@app.rpc(
    service_name='com.alipay.layotto.{your_appname}.{your_facade_name}}',
    method='sayHello',
    unique_id='test-with-unique-id',  # 可选，同一方法多实现时的唯一标识
    protocol='tr',
)
def say_hello_with_unique_id(request: RpcRequest) -> SofaRpcResponse:
    hello_request = HelloRequest(**request.data)
    response = HelloResponse(message=f"Hello with unique_id, {hello_request.name}!", success=True)
    return SofaRpcResponse(is_error=False, app_response=response)
```

因 Python 模块加载机制，handler 文件需在 `servers/sofa/__init__.py` 中显式导入，装饰器才会执行服务注册：

```python
# servers/sofa/__init__.py
from servers.sofa.rpc.tr import hello_facade  # 显式导入
```

#### @app.rpc 装饰器参数

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- |----|
| `service_name` | str | 是 | 服务全限定名，由 rpc.proto 中的 package + service 组合而成，如 `com.alipay.layotto.{应用名}.HelloFacade` |
| `method` | str | 是 | 方法名，对应 Protobuf 中的 RPC 方法 |
| `unique_id` | str | 否 | 同一方法存在多个实现时的唯一标识 |
| `protocol` | str | 否 | 协议类型，默认 'tr'，还支持 'tri' 协议（triple）|

#### RpcRequest 对象

```python
class RpcRequest:
    service_name: str              # 服务名称
    data: dict                     # 请求体数据，可反序列化为 protobuf message
    metadata: Dict[str, str]       # 请求元数据，包含链路追踪信息等
    content_type: Optional[str]    # 内容类型（可选）
```

#### SofaRpcResponse 对象

```python
class SofaRpcResponse:
    is_error: bool                      # 是否错误，默认 False
    app_response: HessianObject         # 响应对象，会被序列化为 protobuf
    error_msg: Optional[str]            # 错误信息（is_error=True 时使用）
    response_props: Optional[dict]      # 响应附加属性
```

创建示例：

```python
return SofaRpcResponse(is_error=False, app_response=response)
```

### 在TR服务中使用中间件（以DRM为例）

使用中间件前需先在 `application.yaml` 中启用，具体资源配置值（如 ZCache 实例、ZDAS 数据源、OSS Bucket 等）需先在对应平台申请后填入，不可自行编造。

```yaml
module_config:
  drm:
    enabled: true
```

**用法1: 在 handler 中可直接通过 `app.layotto_manager` 访问中间件**

```python
from servers.sofa.app import app

@app.rpc(
      service_name='com.alipay.layotto.{your_appname}.{your_facade_name}',
      method='sayHello',
  )
def say_hello_handler(request: RpcRequest) -> SofaRpcResponse:
    # 获取 DRM 配置
    config = app.layotto_manager.drm_manager.get_drm_config("config_id")
    # ...
```

**用法2: 单独模块中使用中间件**

如需在应用启动时初始化中间件（如订阅 DRM 配置变更），可在 `servers/sofa/` 下创建独立模块，并需在 `servers/sofa/__init__.py` 中显式导入此模块，否则不会执行。

```python
# servers/sofa/drm/drm_init.py（文件名自定义）
from servers.sofa.app import app

@app.layotto_manager.drm_manager.drm_subscribe(["config_id"])
def on_config_change(new_value):
  print(f"Config changed: {new_value}")
```

在 `servers/sofa/__init__.py` 中显式导入此模块：

```python
# servers/sofa/__init__.py
from servers.sofa.drm import drm_init  # 替换为实际模块名
```

其他中间件使用方式参考 [中间件使用指南](10_middleware_usage_guide.md)。

## 验证发布

完成 SOFARPC(TR) 服务开发后，按以下步骤验证服务是否成功发布。

**方式一：Mosn 本地接口（本地开发环境）**

### 1. 启动服务

在项目根目录（与 requirements.txt 同级）执行：

```bash
source .venv/bin/activate   # 激活虚拟环境
cd <应用名>                  # 进入应用目录（如 cd sofapyapp）
python main.py              # 启动服务，后台运行可添加 2>&1 &（需通过 ps aux | grep main.py 查找进程后 kill <pid> 终止）
```

等待日志输出 `Application startup complete.` 表示服务启动成功。

### 2. 检查服务注册状态

通过 Mosn 本地接口查看服务是否成功发布：

```bash
curl 127.0.0.1:34901/api/v1/registryserviceclient | python3 -m json.tool
# 若系统无 python3 命令，改用 python
```

### 3. 解读返回结果

检查 Mosn 状态：`rawclient.Health: "ok"` 表示 Mosn 运行正常。

检查服务发布状态：在 `pub_enabled_config` 中查找你的服务，Key 格式为 `com.alipay.layotto.{app_name}.{facade_name}:1.0@DEFAULT#@#DEFAULT_INSTANCE_ID#@#SOFA`，`registry.enabled: true` 表示服务已成功发布到注册中心。

### 4. 测试服务响应（可选）

可通过客户端调用该服务，验证服务是否正常响应。参见 [TR 调用示例](11_tr_usage_guide.md)。

**方式二：SOFA Portal（DEV/线上环境）**

需用户自行登录 [SOFA Portal](https://sofa.alipay.com/)，切换对应环境后输入应用名或服务名进行查询，在服务查询页面选择 TR 协议类型。