# SofaRPC（TR）服务调用指南

SofaRPC（TR）是蚂蚁集团内部的 RPC 框架。调用 TR 服务需经过配置、安装工具、生成代码、调用服务等步骤。

注意：生成代码位于 `./oneapi/` 目录，`f.yml` 配置变化时需重新生成代码。

## 安装 oneapi-codegen

检查是否已安装：

```bash
oneapi-codegen -v
```

若未安装，详见 [oneapi-codegen 安装指南](09_oneapi_codegen_install_guide.md) 中安装步骤。

## 配置服务订阅

```yaml
# configs/application.yaml
module_config:
  rpc:
    tr:
      sub:
        - "com.alipay.{目标应用名}.facade.{目标接口名}:1.0"  # 接口全限定名:版本号，版本号默认为 1.0
```

## 配置 f.yml

目标文件：在应用包同目录（与main.py同级）查找文件 `f.yml`，没有则创建。

在 [OneAPI 平台](https://oneapi.alipay.com/) 查找目标服务所属应用名。

配置示例：
```yaml
oneapi:
  - appname: chairmosn      # 必填: OneAPI 应用名
    # source: ZAPPINFO     # 可选: 非主站环境需指定
    # tag: master          # 可选: 迭代版本，默认 master
    lang: python           # 必填: 生成语言

    api:
      EchoFacade:          # 服务名
        enable: true       # 必填: 启用生成
        # responseTimeout: 5000        # 可选: 超时时间(ms)
        # uniqueId: xxx                # 可选: 服务唯一标识

  - appname: basementurl
    tag: master
    lang: python
    api:
      URLFacadeV2:
        enable: true
```

## 生成客户端代码

```bash
# 在 f.yml 所在目录执行
oneapi-codegen layotto --target oneapi
```

生成代码位于 `./oneapi/` 目录，例如：

```
oneapi/
├── basementurl/
│   └── URLFacadeV2/
│       ├── __init__.py
│       └── URLFacadeV2.py
└── chairmosn/
    └── EchoFacade/
        ├── __init__.py
        └── EchoFacade.py
```

## 代码调用

### 基础调用

```python
from layotto import get_mosn_client
from oneapi.{appname}.{servicename} import FacadeName

layotto_client = get_mosn_client()
service = FacadeName(layotto_client)

# 调用方法
# req = RequestType(param="value")
# res = service.method_name(req)
```

### 完整示例

```python
from layotto import get_mosn_client
from oneapi.basementurl.URLFacadeV2 import URLFacadeV2
from oneapi.basementurl.URLFacadeV2 import (
    com_alipay_basementurl_facade_ShortenRequest
)

layotto_client = get_mosn_client()
basementurl = URLFacadeV2(layotto_client)

# 构建请求
req = com_alipay_basementurl_facade_ShortenRequest(
    uid="123123",
    app="your-app-name",
    domain="basementurl.test.alipay.net",
    url="https://example.com"
)

# 调用方法
res = basementurl.shorten(req)
print(f"Short URL: {res.short_url}")
```