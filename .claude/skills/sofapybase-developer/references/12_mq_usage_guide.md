# MQ 使用指南

本文档介绍蚂蚁内部三款 MQ 产品（SOFAMQ、SOFAMQX、MsgBroker）的使用方法。

## SOFAMQ

### 配置

```yaml
module_config:
  sofamq:
    enabled: true
```

### 生产者

```python
from layotto import Result, LDCSubMode
from layotto.core.client import set_trace_context
import uuid

@manager.sofamq_manager.publisher(
    topic="topic_name",          # 必填: 消息主题
    group="producer_group",      # 必填: 生产者组
    tag="",                      # 可选: 消息标签
    properties={"key": "value"}, # 可选: 默认 metadata
)
def send_message(message_data):
    return message_data  # 支持 str 或 bytes

# 发送消息
send_message("Hello")

# 带 properties 发送（覆盖默认值）
send_message("Hello", properties={"key": "override"})

# 带 Trace Context 发送
trace_id = str(uuid.uuid4())
set_trace_context(trace_id, "0.1.2.3")
send_message("message with trace")
set_trace_context(None, None)
```

### 消费者

```python
from layotto import Result, LDCSubMode

@manager.sofamq_manager.subscriber(
    topic="topic_name",          # 必填
    group="consumer_group",      # 必填
    tag="",                      # 可选: 消息标签
    concurrent=20,               # 可选: 并发数，默认 20
    ldc_mode=LDCSubMode.LOCAL,   # 可选: LOCAL/RANDOM/ROUND_ROBIN
)
def message_handler(event):
    print(f"Received: {event.msg_data}")
    return Result.SUCCESS
```

event 对象包含消息相关的属性：msg_topic、msg_data（bytes 类型）、msg_tags、sofa_trace_id、sofa_rpc_id、msg_size、msg_producer_group_id、msg_flag、msg_deliver_count、msg_content_type、msg_consumer_group_id、msg_consume_start_time、msg_component、msg_broker_name、msg_born_time、msg_born_host、msg_queue_offset、msg_queue_id，以及 get_property(key, default=None) 方法。其他属性为 str 类型。

## SOFAMQX

### 配置

```yaml
module_config:
  sofamqx:
    enabled: true
```

### 生产者

```python
@manager.sofamqx_manager.publisher(
    topic="topic_name",
    group="producer_group",
    endpoint="http://endpoint.url",  # 必填
    tags=["tag1", "tag2"],           # 可选: 标签数组
    properties={"key": "value"},
)
def send_message(message_data):
    return message_data  # 支持 str 或 bytes
```

### 消费者

```python
@manager.sofamqx_manager.subscriber(
    topic="topic_name",
    group="consumer_group",
    endpoint="http://endpoint.url",  # 必填
    tags=None,                       # 可选: 标签数组
    concurrent=20,
)
def message_handler(event):
    return Result.SUCCESS
```

event 对象包含属性：msg_topic、msg_data（bytes 类型）、msg_tags、sofa_trace_id、sofa_rpc_id、msg_content_type、msg_consumer_group_id、msg_component、msg_born_time。

## MsgBroker

### 配置

```yaml
module_config:
  msgbroker:
    enabled: true
```

### 生产者

```python
@manager.msgbroker_manager.publisher(
    topic="topic_name",
    group="producer_group",
    event_code="EVENT_CODE",     # 必填: 事件编码
    properties={"key": "value"},
)
def send_message(message_data):
    return message_data  # 支持任意 JSON 可序列化对象
```

### 消费者

```python
@manager.msgbroker_manager.subscriber(
    topic="topic_name",
    group="consumer_group",
    event_code="EVENT_CODE",     # 必填
    concurrent=20,
)
def message_handler(event):
    return Result.SUCCESS
```

event 对象包含属性：msg_topic、msg_data（bytes 类型）、msg_tags、sofa_trace_id、sofa_rpc_id、msg_size、msg_producer_group_id、msg_flag、msg_deliver_count、msg_content_type、msg_consumer_group_id、msg_consume_start_time、msg_component、msg_broker_name、msg_born_time、msg_born_host，以及 get_property(key, default=None) 方法。系统属性包含 GROUP_ID（消费者组）和 EVENTCODE（事件编码）。相比 SOFAMQ，缺少 msg_queue_offset 和 msg_queue_id。

## 产品对比

| 产品 | 特有必填参数 | tag | 消息类型 | LDC 模式 |
|------|-------------|-----|---------|---------|
| SOFAMQ | 无 | `tag` (str) | str/bytes | 支持 |
| SOFAMQX | `endpoint` | `tags` (List[str]) | str/bytes | 不支持 |
| MsgBroker | `event_code` | 无 | JSON 对象 | 不支持 |