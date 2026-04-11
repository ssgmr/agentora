//! ChronicleStore 持久化编年史
//!
//! Markdown 文件：CHRONICLE.md（Agent 编年史）+ WORLD_SEED.md（世界认知）
//! 冻结快照模式：decision 开始时注入 prompt，中途不变

use std::path::{Path, PathBuf};
use std::fs;
use agentora_ai::config::MemoryConfig;

const ENTRY_DELIMITER: &str = "§";

/// ChronicleStore 编年史存储
#[derive(Debug)]
pub struct ChronicleStore {
    base_path: PathBuf,
    chronicle_limit: usize,
    world_seed_limit: usize,
    chronicle_content: String,
    world_seed_content: String,
}

impl ChronicleStore {
    /// 从配置初始化
    pub fn from_config(agent_id: &str, config: &MemoryConfig) -> Self {
        let base_path = expand_agent_path(agent_id);
        Self {
            base_path,
            chronicle_limit: config.chronicle_limit,
            world_seed_limit: config.world_seed_limit,
            chronicle_content: String::new(),
            world_seed_content: String::new(),
        }
    }

    /// 使用默认配置初始化（向后兼容）
    pub fn with_defaults(agent_id: &str) -> Self {
        Self::from_config(agent_id, &MemoryConfig::default())
    }

    pub fn new(agent_id: &str) -> Self {
        Self::with_defaults(agent_id)
    }

    /// 加载编年史内容
    pub fn load(&mut self) -> Result<(), std::io::Error> {
        // 崩溃恢复：删除残留的.tmp 文件
        self.cleanup_temp_files()?;

        // 创建目录（若不存在）
        fs::create_dir_all(&self.base_path)?;

        // 加载 CHRONICLE.md
        let chronicle_path = self.base_path.join("CHRONICLE.md");
        self.chronicle_content = self.load_file_with_backup(&chronicle_path)?;

        // 加载 WORLD_SEED.md
        let world_seed_path = self.base_path.join("WORLD_SEED.md");
        self.world_seed_content = self.load_file_with_backup(&world_seed_path)?;

        Ok(())
    }

    /// 加载文件，若损坏则备份并创建新文件
    fn load_file_with_backup(&self, path: &Path) -> Result<String, std::io::Error> {
        if !path.exists() {
            // 文件不存在，创建空文件
            fs::write(path, "")?;
            return Ok(String::new());
        }

        match fs::read_to_string(path) {
            Ok(content) => Ok(content),
            Err(e) => {
                // 文件损坏或权限错误，备份并创建新文件
                let backup_path = path.with_extension("md.bak");
                fs::rename(path, &backup_path)?;
                fs::write(path, "")?;
                tracing::warn!("File {:?} corrupted, backed up to {:?}: {}", path, backup_path, e);
                Ok(String::new())
            }
        }
    }

    /// 清理残留的临时文件
    fn cleanup_temp_files(&self) -> Result<(), std::io::Error> {
        let tmp_path = self.base_path.join("CHRONICLE.md.tmp");
        if tmp_path.exists() {
            fs::remove_file(tmp_path)?;
            tracing::debug!("Cleaned up stale temp file");
        }

        let world_seed_tmp = self.base_path.join("WORLD_SEED.md.tmp");
        if world_seed_tmp.exists() {
            fs::remove_file(world_seed_tmp)?;
        }

        Ok(())
    }

    /// 解析 Agent 目录路径
    pub fn get_base_path(&self) -> &Path {
        &self.base_path
    }

    /// 获取冻结快照（用于当前决策）
    pub fn get_snapshot(&self) -> ChronicleSnapshot {
        ChronicleSnapshot {
            chronicle: self.chronicle_content.clone(),
            world_seed: self.world_seed_content.clone(),
        }
    }

    /// 添加编年史 entry
    pub fn add_entry(&mut self, tick: u32, content: &str) {
        let entry = format!("{}[tick {}] {}\n", ENTRY_DELIMITER, tick, content);
        self.chronicle_content.push_str(&entry);

        // 超限截断
        if self.chronicle_content.chars().count() > self.chronicle_limit {
            self.truncate_oldest();
        }
    }

    /// 添加世界认知 entry
    pub fn add_world_seed_entry(&mut self, tick: u32, content: &str) {
        let entry = format!("{}[tick {}] {}\n", ENTRY_DELIMITER, tick, content);
        self.world_seed_content.push_str(&entry);

        // 超限截断
        if self.world_seed_content.chars().count() > self.world_seed_limit {
            self.truncate_world_seed_oldest();
        }
    }

    /// 截断最旧的 entry（CHRONICLE.md）
    fn truncate_oldest(&mut self) {
        loop {
            // 按§分隔符分割 entries
            let entries: Vec<&str> = self.chronicle_content
                .split(ENTRY_DELIMITER)
                .filter(|s| !s.is_empty())
                .collect();

            if entries.is_empty() || self.chronicle_content.chars().count() <= self.chronicle_limit {
                break;
            }

            // 移除最旧的 entry 并重新构建
            let new_content = entries[1..].join(ENTRY_DELIMITER);
            self.chronicle_content = new_content;
        }
    }

    /// 截断最旧的 entry（WORLD_SEED.md）
    fn truncate_world_seed_oldest(&mut self) {
        loop {
            let entries: Vec<&str> = self.world_seed_content
                .split(ENTRY_DELIMITER)
                .filter(|s| !s.is_empty())
                .collect();

            if entries.is_empty() || self.world_seed_content.chars().count() <= self.world_seed_limit {
                break;
            }

            // 移除最旧的 entry 并重新构建
            let new_content = entries[1..].join(ENTRY_DELIMITER);
            self.world_seed_content = new_content;
        }
    }

    /// 原子写入：先写入临时文件，再 rename 覆盖
    pub fn atomic_write(&self) -> Result<(), std::io::Error> {
        // 安全扫描
        Self::security_scan(&self.chronicle_content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{:?}", e)))?;
        Self::security_scan(&self.world_seed_content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{:?}", e)))?;

        // 写入 CHRONICLE.md
        let chronicle_path = self.base_path.join("CHRONICLE.md");
        let chronicle_tmp = self.base_path.join("CHRONICLE.md.tmp");
        fs::write(&chronicle_tmp, &self.chronicle_content)?;
        fs::rename(&chronicle_tmp, &chronicle_path)?;

        // 写入 WORLD_SEED.md
        let world_seed_path = self.base_path.join("WORLD_SEED.md");
        let world_seed_tmp = self.base_path.join("WORLD_SEED.md.tmp");
        fs::write(&world_seed_tmp, &self.world_seed_content)?;
        fs::rename(&world_seed_tmp, &world_seed_path)?;

        Ok(())
    }

    /// 安全扫描（检测 prompt injection 等威胁）
    pub fn security_scan(content: &str) -> Result<(), SecurityError> {
        let threat_patterns = [
            "ignore previous instructions",
            "you are now",
            "override rules",
        ];

        for pattern in &threat_patterns {
            if content.to_lowercase().contains(pattern) {
                return Err(SecurityError::ThreatDetected(pattern.to_string()));
            }
        }

        // 检测零宽字符
        for ch in content.chars() {
            if ch == '\u{200B}' || ch == '\u{200C}' || ch == '\u{200D}' {
                return Err(SecurityError::InvisibleUnicode);
            }
        }

        Ok(())
    }

    /// 获取编年史内容
    pub fn get_chronicle(&self) -> &str {
        &self.chronicle_content
    }

    /// 获取世界认知内容
    pub fn get_world_seed(&self) -> &str {
        &self.world_seed_content
    }
}

impl Default for ChronicleStore {
    fn default() -> Self {
        Self::new("default")
    }
}

/// 编年史快照（冻结状态）
#[derive(Debug, Clone)]
pub struct ChronicleSnapshot {
    pub chronicle: String,
    pub world_seed: String,
}

/// 安全扫描错误
#[derive(Debug, Clone)]
pub enum SecurityError {
    ThreatDetected(String),
    InvisibleUnicode,
}

/// 展开 Agent 目录路径
fn expand_agent_path(agent_id: &str) -> PathBuf {
    // 使用 dirs crate 获取主目录，或回退到环境变量
    let home_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())));
    home_dir
        .join(".agentora")
        .join("agents")
        .join(agent_id)
}
