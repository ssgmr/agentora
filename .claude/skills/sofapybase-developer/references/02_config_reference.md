# 配置机制

框架通过多环境配置文件分离实现环境差异化，高优先级配置会覆盖相同字段。这种设计便于在不同环境（开发、测试、预发、生产）间切换而无需修改核心配置。

## 加载机制

配置合并优先级从高到低：`application-{env}.yaml`（环境配置，如 dev/prepub/gray/prod/test/sim）优先于 `application.yaml`（基础配置，所有环境共享，必需）。按需创建环境配置文件即可。

## 根级配置说明

根级配置通常保持默认，有特殊需求再调整。

```yaml
app_name: "your_app_name"           # 应用名称（必填，脚手架初始化时已设置）

# 以下配置通常保持默认
enable_sidecar: true                # 启用 Sidecar，默认 true
workers: 1                          # 工作进程数，默认 1
health_check:                       # 框架健康检查端口，默认 9500
  port: 9500


sidecar_config:                     # Sidecar 配置
  host: "localhost"
  max_send_message_length: 4194304  # 4MB
  max_receive_message_length: 4194304

log_config:
  trace_log_dir: ''
  log_level: 'INFO'
  log_dir: ''
```

## module_config 配置

以下展示所有支持的中间件配置，实际开发按需配置用到的模块即可。请严格遵循以下配置结构，不要自行添加未列出的字段。中间件的资源配置值（如 ZCache 实例、ZDAS 数据源、OSS Bucket 等）需先在对应平台申请后填入，不可自行编造。

```yaml
module_config:
  rpc:
    tr:
      sub:
        - "com.alipay.{target_app_name}.facade.{service_name}:1.0"

  zcache:
    enabled: true
    caches:
      - cache_name: "xxx"           # ZCache 实例名
        route_type: "G"             # G=GZone, R=RZone, C=CZone

  zdas:
    enabled: true
    datasources:
      - database: "xxx"             # 数据库名
        user: "xx:appcenter:xx"     # ZDAS 认证用户名
        password: ""                # 密码（可选）

  ddsoss:
    enabled: true
    storage:
      - bucket: "xxx"               # OSS Bucket（与资源平台申请的一致）
        data_source: "xxx"          # 数据源（与 SOFAPortal 配置的一致）
        datasource_version: "v1"    # 版本号（与 SOFAPortal 配置的一致）

  sofamq:
    enabled: true
  sofamqx:
    enabled: true
  msgbroker:
    enabled: true

  drm:
    enabled: true
  maya:
    enabled: true
  flowcontrol:
    enabled: true

  mist:
    enabled: true
    tenant: "ALIPAY"                # 租户，支付宝主站为 ALIPAY
    mode: "dev"                     # 与 Mist 平台配置的 mode 一致
#    app_name: ""                    # 默认继承全局，需与机密平台上的授权应用一致（如不同可显式覆盖）
#    secret_server: ""               # mist服务端地址，主站默认填充，非主站环境参考 https://yuque.antfin.com/yuxuezhi.yxz/ki7p3h/mqrgcm#lQ2wd
#    antvip_url: ""                  # antvip服务端地址，主站默认填充
#    mesh_url: ""                    # 鉴权地址，主站默认填充

  lock:
    source_name: "cacheInstance"    # ZCache 实例名（必须为 GZone 资源）
#    app_name: ""                    # 默认继承全局

  web:
    host: "0.0.0.0"                 # 绑定地址，默认 0.0.0.0
    port: 8888                      # 服务端口，默认 8888
#    health_check_endpoint: "/health"  # Web服务健康检查端点
    start: "servers.web.app:app"    # FastAPI 应用导入路径

  sofa:
    start: "servers.sofa.app:app"   # SOFA 服务启动入口

  mcp:
    sub:
      - mcp.ant.faas.deriskMcpServerGroup.mcpAntscheduler: {"mesh_vip_address": "faasgw-pool:8080"}
      - mcp.ant.sofadoc.sofadocmcpserver: {}
    pub:
      - service_name: "your_mcp_server_name"  # MCP 服务名称
        description: "your_mcp_description"
        open_secaspect: true        # 开启安全切面，默认 true
        protocol: 0                 # 0=SSE, 1=Streamable, 2=Streamable Stateless, 3=SSE+Streamable, 4=SSE+Streamable Stateless
        json_response: false
#        path: "/mcp1/"              # 自定义路径，默认为空，多MCP Server 时必填
    start: "servers.mcp.app"
```

各中间件详细用法见 [中间件使用指南](10_middleware_usage_guide.md)