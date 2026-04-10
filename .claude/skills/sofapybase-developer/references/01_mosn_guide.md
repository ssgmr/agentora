# Layotto(Mosn) 本地安装

Layotto(Mosn) 是服务网格运行时，为应用提供 RPC 调用、MCP调用、配置管理、服务发现等核心能力。Meshboot 是其配套的命令行管理工具。

## 安装 Meshboot

```bash
bash <(curl -sL https://t.tb.cn/_meshboot_install)
```

## 启动 Mosn

主站环境使用以下命令启动，其中 `-a` 参数指定应用名称，需与 `application.yaml` 中的 `app_name` 保持一致：

```bash
meshboot start -m binary -a <应用名>
```

示例：`meshboot start -m binary -a sofapyapp`

非主站环境请参考 [Layotto 安装文档](https://yuque.antfin.com/sofa-open/cnar/quickstart-install-layotto#CoVQB) 启动 Mosn。

## 检查状态

```bash
meshboot status -m binary
```

启动成功时输出 `{"status":"UP","components":{...}}`，仅需检查顶层 `status` 字段即可，`components` 中的组件状态仅供参考。若输出 `{"status":"DOWN",...}` 或 `MOSN is not started ...`，说明状态异常或未启动，请执行重启命令。

## 停止 Mosn

```bash
meshboot stop -m binary
```

## 重启 Mosn

```bash
meshboot restart -m binary -a <应用名>
```

示例：`meshboot restart -m binary -a sofapyapp`