//! 日志配置和初始化
//!
//! 从 config/log.toml 加载日志参数

use std::sync::Once;

// 日志初始化器（只执行一次）
static LOG_INIT: Once = Once::new();
static mut LOG_GUARD: Option<tracing_appender::non_blocking::WorkerGuard> = None;

/// 初始化日志系统（只执行一次）
pub fn init_logging() {
    godot::global::print(&[godot::prelude::Variant::from("[Logger] init_logging called")]);
    LOG_INIT.call_once(|| {
        if let Err(e) = try_init_logging() {
            eprintln!("[Logger] 初始化失败: {}", e);
        }
    });
}

fn try_init_logging() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tracing_subscriber::{fmt, EnvFilter, Layer, prelude::*};
    use tracing_subscriber::fmt::time::LocalTime;
    use time::macros::format_description;

    // 加载日志配置
    let log_cfg = LogConfig::load(&resolve_config_path("config/log.toml"));

    // 日志目录：从配置文件读取，相对于当前工作目录
    let log_dir = std::path::Path::new(&log_cfg.log_dir);
    std::fs::create_dir_all(log_dir)?;

    // 构建 EnvFilter
    let mut filter_str = log_cfg.file_level.clone();
    for (target, level) in &log_cfg.targets {
        filter_str.push_str(&format!(",{}={}", target, level));
    }

    let console_filter = EnvFilter::try_new(if log_cfg.console_enabled {
        &log_cfg.console_level
    } else {
        "off"
    }).unwrap_or_else(|_| EnvFilter::new("info"));
    let file_filter = EnvFilter::try_new(if log_cfg.file_enabled {
        &filter_str
    } else {
        "off"
    }).unwrap_or_else(|_| EnvFilter::new("debug"));

    let rotation = match log_cfg.rotation.as_str() {
        "hourly" => tracing_appender::rolling::Rotation::HOURLY,
        "never" => tracing_appender::rolling::Rotation::NEVER,
        _ => tracing_appender::rolling::Rotation::DAILY,
    };
    let file_appender = tracing_appender::rolling::RollingFileAppender::new(
        rotation,
        log_dir,
        "agentora.log",
    );
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let time_format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]");
    let local_timer = LocalTime::new(time_format);

    let console_layer = fmt::layer()
        .with_target(false)
        .with_ansi(false)
        .with_timer(local_timer.clone())
        .with_writer(std::io::stdout)
        .with_filter(console_filter);
    let file_layer = fmt::layer()
        .with_target(true)
        .with_ansi(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_timer(local_timer)
        .with_writer(non_blocking)
        .with_filter(file_filter);

    let subscriber = tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer);

    tracing::subscriber::set_global_default(subscriber)?;

    // 保持 guard 存活
    unsafe { LOG_GUARD = Some(guard); }

    godot::global::print(&[godot::prelude::Variant::from(format!(
        "[Logger] 日志已初始化 [{}] → {}",
        log_cfg.file_level, log_cfg.log_dir
    ))]);

    Ok(())
}

/// 日志配置（简化版）
#[derive(Debug, Clone, serde::Deserialize)]
struct LogConfigFile {
    log: Option<LogSection>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct LogSection {
    console_enabled: Option<bool>,
    console_level: Option<String>,
    file_enabled: Option<bool>,
    file_level: Option<String>,
    log_dir: Option<String>,
    rotation: Option<String>,
    targets: Option<std::collections::HashMap<String, String>>,
}

/// 日志配置（运行时使用的扁平结构）
#[derive(Debug, Clone)]
struct LogConfig {
    console_enabled: bool,
    console_level: String,
    file_enabled: bool,
    file_level: String,
    log_dir: String,
    rotation: String,
    targets: std::collections::HashMap<String, String>,
}

/// 解析配置文件路径：优先当前工作目录相对路径，再 fallback 到 exe 所在目录
fn resolve_config_path(relative_path: &str) -> String {
    let cwd_path = std::path::Path::new(relative_path);
    if cwd_path.exists() {
        return relative_path.to_string();
    }
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let exe_relative = exe_dir.join(relative_path);
            if exe_relative.exists() {
                return exe_relative.to_string_lossy().to_string();
            }
        }
    }
    relative_path.to_string()
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            console_enabled: true,
            console_level: "info".to_string(),
            file_enabled: true,
            file_level: "debug".to_string(),
            log_dir: "logs".to_string(),
            rotation: "daily".to_string(),
            targets: std::collections::HashMap::new(),
        }
    }
}

impl LogConfig {
    fn load(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                match toml::from_str::<LogConfigFile>(&content) {
                    Ok(file) => {
                        let mut cfg = Self::default();
                        if let Some(log) = file.log {
                            if let Some(v) = log.console_enabled { cfg.console_enabled = v; }
                            if let Some(v) = log.console_level { cfg.console_level = v; }
                            if let Some(v) = log.file_enabled { cfg.file_enabled = v; }
                            if let Some(v) = log.file_level { cfg.file_level = v; }
                            if let Some(v) = log.log_dir { cfg.log_dir = v; }
                            if let Some(v) = log.rotation { cfg.rotation = v; }
                            if let Some(v) = log.targets { cfg.targets = v; }
                        }
                        cfg
                    }
                    Err(e) => {
                        eprintln!("[Logger] log.toml 解析失败 ({}), 使用默认配置", e);
                        Self::default()
                    }
                }
            }
            Err(e) => {
                eprintln!("[Logger] log.toml 未找到 ({}), 使用默认配置", e);
                Self::default()
            }
        }
    }
}