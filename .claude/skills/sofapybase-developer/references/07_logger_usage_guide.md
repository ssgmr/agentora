# 日志使用指南

## 配置

在 `application.yaml` 中配置日志相关参数：

```yaml
log_config:
  trace_log_dir: ''      # 链路追踪日志目录，默认 ~/logs/tracelog
  log_level: 'INFO'      # 日志级别：DEBUG/INFO/WARNING/ERROR/CRITICAL
  log_dir: ''            # 业务日志目录，默认 ~/logs/sofapy，支持 stdout/stderr
```

## 使用方法

通过 `get_logger` 函数获取 logger 实例，传入的名称将作为日志文件名（如传入 `mcpserver` 则日志输出到 `mcpserver.log`）。不同模块的日志分离到不同文件，便于独立管理和分析。

```python
from sofapy_base.logger.logger import get_logger

logger = get_logger("mcpserver")

logger.debug("调试信息: %s", debug_data)
logger.info("普通信息: %s", info_data)
logger.warning("警告信息: %s", warning_data)
logger.error("错误信息: %s", error_data)
logger.critical("严重错误: %s", critical_data)
```