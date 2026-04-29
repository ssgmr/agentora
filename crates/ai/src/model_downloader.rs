//! 模型下载模块
//!
//! 支持从 ModelScope/HuggingFace CDN 下载 GGUF 模型，提供进度信号。

use reqwest::Client;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::io::AsyncWriteExt;
use thiserror::Error;

/// 下载错误类型
#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("网络请求失败: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("文件写入失败: {0}")]
    IoError(#[from] std::io::Error),

    #[error("下载被取消")]
    Cancelled,

    #[error("所有 CDN 源均失败")]
    AllCdnsFailed,
}

/// 下载进度信息
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// 已下载 MB
    pub downloaded_mb: f64,
    /// 总大小 MB
    pub total_mb: f64,
    /// 下载速度 MB/s
    pub speed_mbps: f64,
    /// 进度百分比 (0-100)
    pub percent: f64,
}

/// 预置模型信息
#[derive(Debug, Clone)]
pub struct ModelEntry {
    /// 模型名称
    pub name: String,
    /// 文件名
    pub filename: String,
    /// 大小 MB
    pub size_mb: u32,
    /// 描述
    pub description: String,
    /// 主 CDN（ModelScope）
    pub primary_url: String,
    /// 备用 CDN（HuggingFace）
    pub fallback_url: String,
}

/// 获取可用模型列表
pub fn get_available_models() -> Vec<ModelEntry> {
    vec![
        ModelEntry {
            name: "Qwen3.5-2B-Q4_K_M".to_string(),
            filename: "Qwen3.5-2B-Q4_K_M.gguf".to_string(),
            size_mb: 1500,
            description: "Qwen3.5 2B Q4量化".to_string(),
            primary_url: "https://modelscope.cn/models/unsloth/Qwen3.5-2B-GGUF/resolve/master/Qwen3.5-2B-Q4_K_M.gguf".to_string(),
            fallback_url: "https://huggingface.co/unsloth/Qwen3.5-2B-GGUF/resolve/main/Qwen3.5-2B-Q4_K_M.gguf".to_string(),
        },
        ModelEntry {
            name: "gemma-4-E2B-it-Q4".to_string(),
            filename: "gemma-4-E2B-it-Q4_K_M.gguf".to_string(),
            size_mb: 1400,
            description: "Google Gemma 4 2B Q4量化".to_string(),
            primary_url: "https://modelscope.cn/models/lmstudio-community/gemma-4-E2B-it-GGUF/resolve/master/gemma-4-E2B-it-Q4_K_M.gguf".to_string(),
            fallback_url: "https://huggingface.co/lmstudio-community/gemma-4-E2B-it-GGUF/resolve/main/gemma-4-E2B-it-Q4_K_M.gguf".to_string(),
        }
    ]
}

/// 模型下载器
pub struct ModelDownloader {
    client: Client,
    cancel_flag: Arc<std::sync::atomic::AtomicBool>,
}

impl ModelDownloader {
    /// 创建下载器
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap_or_else(|_| Client::new()),
            cancel_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// 获取可用模型列表
    pub fn get_available_models() -> Vec<ModelEntry> {
        get_available_models()
    }

    /// 取消当前下载
    pub fn cancel(&self) {
        self.cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// 重置取消标志
    pub fn reset_cancel(&self) {
        self.cancel_flag.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// 流式下载模型
    ///
    /// # Arguments
    ///
    /// * `url` - 下载 URL
    /// * `dest` - 目标路径
    /// * `progress_tx` - 进度信号发送器
    ///
    /// # Returns
    ///
    /// 成功返回目标路径，失败返回错误
    pub async fn download(
        &self,
        url: &str,
        dest: &Path,
        progress_tx: mpsc::Sender<DownloadProgress>,
    ) -> Result<PathBuf, DownloadError> {
        self.reset_cancel();

        // 发送请求
        let response = self.client.get(url).send().await?;

        let total_size = response.content_length().unwrap_or(0) as f64 / 1_048_576.0;
        let mut downloaded: f64 = 0.0;
        let mut last_time = std::time::Instant::now();
        let mut last_downloaded: f64 = 0.0;

        // 确保目标目录存在
        if let Some(parent) = dest.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        // 创建临时文件
        let temp_path = dest.with_extension("tmp");
        let mut file = tokio::fs::File::create(&temp_path).await?;

        // 流式下载
        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;

        while let Some(chunk) = stream.next().await {
            // 检查取消标志
            if self.cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
                // 清理临时文件
                file.flush().await?;
                tokio::fs::remove_file(&temp_path).await.ok();
                return Err(DownloadError::Cancelled);
            }

            let chunk = chunk?;
            file.write_all(&chunk).await?;

            downloaded += chunk.len() as f64 / 1_048_576.0;

            // 计算速度（每 0.5 秒发送一次进度）
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(last_time).as_secs_f64();
            if elapsed > 0.5 {
                let speed = (downloaded - last_downloaded) / elapsed;
                let percent = if total_size > 0.0 {
                    downloaded / total_size * 100.0
                } else {
                    0.0
                };

                last_time = now;
                last_downloaded = downloaded;

                // 发送进度
                let progress = DownloadProgress {
                    downloaded_mb: downloaded,
                    total_mb: total_size,
                    speed_mbps: speed,
                    percent,
                };

                // 发送失败不影响下载继续
                progress_tx.send(progress).await.ok();
            }
        }

        file.flush().await?;

        // 重命名为最终文件
        tokio::fs::rename(&temp_path, dest).await?;

        Ok(dest.to_path_buf())
    }

    /// 尝试从多个 CDN 下载
    ///
    /// 先尝试主 CDN，失败后尝试备用 CDN
    pub async fn download_with_fallback(
        &self,
        model: &ModelEntry,
        dest_dir: &Path,
        progress_tx: mpsc::Sender<DownloadProgress>,
    ) -> Result<PathBuf, DownloadError> {
        let dest = dest_dir.join(&model.filename);

        // 尝试主 CDN
        tracing::info!("尝试从 ModelScope 下载: {}", model.primary_url);
        match self.download(&model.primary_url, &dest, progress_tx.clone()).await {
            Ok(path) => return Ok(path),
            Err(DownloadError::Cancelled) => return Err(DownloadError::Cancelled),
            Err(e) => {
                tracing::warn!("ModelScope 下载失败: {}, 尝试 HuggingFace", e);
            }
        }

        // 尝试备用 CDN
        tracing::info!("尝试从 HuggingFace 下载: {}", model.fallback_url);
        self.download(&model.fallback_url, &dest, progress_tx).await
    }
}

impl Default for ModelDownloader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available_models() {
        let models = get_available_models();
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.name.contains("Qwen")));
    }

    #[test]
    fn test_progress_calculation() {
        let progress = DownloadProgress {
            downloaded_mb: 500.0,
            total_mb: 1500.0,
            speed_mbps: 5.0,
            percent: 33.33,
        };
        assert_eq!(progress.percent, 33.33);
    }
}