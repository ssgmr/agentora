# 中间件使用指南

本文档介绍 SOFAPy 框架支持的各类中间件配置与使用方式。中间件的资源配置值（如 ZCache 实例、ZDAS 数据源、OSS Bucket 等）需先在对应平台申请后填入配置，不可自行编造。

## SofaRPC(TR) 调用

详见 [TR 服务调用指南](11_tr_usage_guide.md)。

## ZCache 分布式缓存

ZCache 是蚂蚁内部的分布式缓存服务，支持 String、Hash、List、Set、ZSet 等数据结构操作。

### 配置

```yaml
module_config:
  zcache:
    enabled: true
    caches:
      - cache_name: "myCacheInstance"  # 代码中通过此名称获取实例
        route_type: "G"                # G=GZone, R=RZone, C=CZone
```

### 代码使用

```python
from sofapy_base.app.layotto_manager import get_layotto_manager

manager = get_layotto_manager()
cache = manager.zcache_manager.get_zcache("cache_name")

```

具体的 String、Hash、List、Set、ZSet 等 API 操作详见 [ZCache 使用指南](13_zcache_usage_guide.md)。

## ZDAS 数据库服务

ZDAS 是蚂蚁内部的数据库访问服务，用法与 MySQL-python（pymysql）一致。

### 配置

```yaml
module_config:
  zdas:
    enabled: true
    datasources:
      - database: "my_database"        # 代码中引用此名称
        user: "xx:sofaappcenter:xx"    # ZDAS 认证用户名
        password: ""                   # 密码（可选）
```

### 代码使用

```python
try:
    with manager.zdas_manager.get_connection("my_database") as connection: # 获取数据库名建立连接
        with connection.cursor() as cursor:
            query = "SELECT * FROM student WHERE score > %s"
            cursor.execute(query, (100,))
except Exception as e:
    pass
```

## DDSOSS 对象存储服务

DDSOSS 是蚂蚁内部的对象存储服务。

### 配置

```yaml
module_config:
  ddsoss:
    enabled: true
    storage:
      - bucket: "my-bucket"            # bucket 申请参考 https://yuque.antfin.com/middleware/cloudstoragemng/create-bucket
        data_source: "my_oss_ds"       # 与 SOFAPortal 配置一致
        datasource_version: "v1"       # 与 SOFAPortal 配置一致
```

### 代码使用

```python
bucket = manager.ddsoss_manager.get_oss_bucket(bucket="bucket_name")

bucket.put_object("path/to/file", data)
data = bucket.get_object("path/to/file")
bucket.delete_object("path/to/file")
```

追加上传、对象标签、ACL、签名 URL、分片上传等操作详见 [DDSOSS 使用指南](14_ddsoss_usage_guide.md)。

## DRM 动态配置

DRM 支持配置热更新和变更订阅，适用于需要动态调整的业务参数。`get_drm_config` 方法返回 `DRMConfiguration` 对象（导入：`from layotto import DRMConfiguration`），包含 `key`（配置项 key）、`value`（配置值）、`version`（配置版本号）三个属性。

### 配置

```yaml
module_config:
  drm:
    enabled: true
```

### 代码使用

```python
# 读取配置
config = manager.drm_manager.get_drm_config("your_config_key")
print(f"Key: {config.key}")
print(f"Value: {config.value}")
print(f"Version: {config.version}")

# 装饰器订阅配置变更（推荐）
subscribe_ids = [
    "Alipay.chairmson:name=com.alipay.chairmosn.iactest.test,version=3.0@DRM",
]

@manager.drm_manager.drm_subscribe(subscribe_ids)
def on_change(new_value):
    # new_value 类型为 DRMConfiguration
    print(f"Config changed: {new_value.key} = {new_value.value}")

# 动态订阅（不推荐使用）
manager.drm_manager.subscribe(subscribe_ids, handler)
```

## Mist 机密管理

Mist 用于管理应用机密信息，如数据库密码、API 密钥等敏感数据。

### 配置

```yaml
module_config:
  mist:
    enabled: true
    tenant: "ALIPAY"                # 租户，支付宝主站为 ALIPAY
    mode: "dev"                     # 与 Mist 平台配置的 mode 一致
    # app_name                      # 默认继承全局，需与机密平台上的授权应用一致（如不同可显式覆盖）
    # secret_server                 # mist服务端地址，主站默认填充
    # antvip_url                    # antvip服务端地址，主站默认填充
    # mesh_url                      # 鉴权地址，主站默认填充
```

平台配置参考 [各租户和站点地址](https://yuque.antfin.com/antcsp/doc/sitelist) 及 [非主站环境的配置](http://yuque.antfin.com/yuxuezhi.yxz/ki7p3h/mqrgcm#lQ2wd)。

### 代码使用

```python
secret = manager.mist_manager.get_secret(secret_name="secret_key_name")
secret_user = secret.secret_user
secret_value = secret.secret_value
```

## Flowcontrol 流量控制

Flowcontrol 用于实现接口级别的限流和熔断保护。

### 配置

```yaml
module_config:
  flowcontrol:
    enabled: true
```

### 代码使用

```python
# HTTP 接口便捷方法
resp = manager.flowcontrol_manager.should_block_http(
    path="/api/v1/hello",
    method="GET"
)

# 通用流控请求
from layotto import (
    FlowControlRequest,
    FlowControlRequestResourceType,
    FlowControlRequestTrafficType,
    build_http_service_id
)

request = FlowControlRequest(
    service_id=build_http_service_id("/api/v1/hello", "get"),
    resource_type=FlowControlRequestResourceType.RES_TYPE_WEB,
    traffic_type=FlowControlRequestTrafficType.IN,
)
resp = manager.flowcontrol_manager.should_block(request)
```

## 分布式锁

分布式锁基于 ZCache 实现，用于解决分布式环境下的资源竞争问题。

### 配置

```yaml
module_config:
  lock:
    source_name: 'cacheInstanceName'   # ZCache 实例名（必须为 GZone）
```

### 代码使用

```python
resource_id = f'sofapy_test_lock_{datetime.now()}'
lock = manager.lock_manager.get_lock(resource_id=resource_id)

# 阻塞式获取锁
lock.lock()
lock.unlock()

# 非阻塞式获取锁
acquired = lock.try_lock(
    lease_time_sec=1,    # 租约时长（秒），默认 1s
    wait_time_ms=500,    # 最长等待时间（毫秒），默认 500ms
    interval_ms=10       # 重试间隔（毫秒），默认 10ms
)
```

## Maya AI 推理服务

Maya 是蚂蚁内部的 AI 推理服务，支持普通推理和流式推理。

### 配置

```yaml
module_config:
  maya:
    enabled: true
```

### 代码使用

```python
from layotto import (
    Debug, InferenceRequest, Item, LBConfig,
    LoadBalancerType, MayaConfig, TensorFeatures, User,
)

# 创建 Item
item = Item()
item.set_item_id("123456789")
tensor_features = TensorFeatures()
tensor_features.set_string_values(["feature1"])
item.set_tensor_features({"query1": tensor_features})

# 创建 User
user = User()
user.user_id = "123456789"

# 创建配置
config = MayaConfig()
config.request_time_out = 100000

lb_config = LBConfig()
lb_config.allow_cross_city = False
lb_config.lb_policy = LoadBalancerType.ROUND_ROBIN

# 普通推理
request = InferenceRequest(
    scene_name="scene_name",
    chain_name="v1",
    items=[item],
    config=config,
    user=user,
    debug=Debug.OPEN,
    lb_config=lb_config,
)
resp = manager.maya_manager.inference(request)

# 流式推理
for resp in manager.maya_manager.stream_inference(request):
    print(f"Result: {resp.items[0].score}")
```

## 消息队列

SOFAMQ、SOFAMQX、MsgBroker 三款消息队列产品，详见 [消息队列使用指南](12_mq_usage_guide.md)。