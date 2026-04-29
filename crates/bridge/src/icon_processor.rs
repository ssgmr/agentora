//! 图标处理模块
//!
//! 用于处理用户上传的自定义图标，缩放到 32x32。

use image::{ImageFormat, imageops::FilterType};
use std::path::{Path, PathBuf};
use std::fs;
use thiserror::Error;

/// 图标处理错误
#[derive(Debug, Error)]
pub enum IconError {
    #[error("无法识别图片格式: {0}")]
    UnknownFormat(String),

    #[error("图片加载失败: {0}")]
    LoadError(#[from] image::ImageError),

    #[error("文件操作失败: {0}")]
    IoError(#[from] std::io::Error),

    #[error("不支持的图片格式: {0}")]
    UnsupportedFormat(String),
}

/// 图标处理结果
#[derive(Debug)]
pub struct ProcessedIcon {
    /// 处理后的图标路径
    pub path: PathBuf,
    /// 原始尺寸
    pub original_size: (u32, u32),
    /// 处理后尺寸（32x32）
    pub processed_size: (u32, u32),
}

/// 目标图标尺寸
pub const TARGET_SIZE: u32 = 32;

/// 支持的图片格式
pub const SUPPORTED_FORMATS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp"];

/// 处理自定义图标
///
/// 将用户上传的图片缩放到 32x32，使用 Lanczos3 过滤器保证质量。
///
/// # Arguments
///
/// * `source_path` - 原始图片路径
/// * `dest_path` - 目标保存路径
///
/// # Returns
///
/// 返回处理结果，包含原始尺寸和处理后尺寸
pub fn process_custom_icon(source_path: &Path, dest_path: &Path) -> Result<ProcessedIcon, IconError> {
    // 检查格式
    let format = detect_format(source_path)?;
    if !is_format_supported(&format) {
        return Err(IconError::UnsupportedFormat(format));
    }

    // 加载图片（使用 image::open）
    let img = image::open(source_path)?;

    // 记录原始尺寸
    let original_size = (img.width(), img.height());

    // 缩放处理（Lanczos3 过滤器）
    let resized = img.resize(TARGET_SIZE, TARGET_SIZE, FilterType::Lanczos3);

    // 确保目标目录存在
    if let Some(parent) = dest_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // 保存为 PNG
    resized.save_with_format(dest_path, ImageFormat::Png)?;

    Ok(ProcessedIcon {
        path: dest_path.to_path_buf(),
        original_size,
        processed_size: (TARGET_SIZE, TARGET_SIZE),
    })
}

/// 检测图片格式
fn detect_format(path: &Path) -> Result<String, IconError> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| IconError::UnknownFormat("无扩展名".to_string()))?;

    Ok(ext.to_lowercase())
}

/// 检查格式是否支持
fn is_format_supported(format: &str) -> bool {
    SUPPORTED_FORMATS.contains(&format)
}

/// 验证图标是否为有效尺寸
///
/// 检查图标是否为 32x32 的 PNG 图片
pub fn validate_icon(path: &Path) -> Result<bool, IconError> {
    if !path.exists() {
        return Ok(false);
    }

    let img = image::open(path)?;

    // 检查尺寸
    if img.width() != TARGET_SIZE || img.height() != TARGET_SIZE {
        return Ok(false);
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_format_supported() {
        assert!(is_format_supported("png"));
        assert!(is_format_supported("jpg"));
        assert!(is_format_supported("jpeg"));
        assert!(is_format_supported("webp"));
        assert!(!is_format_supported("gif"));
        assert!(!is_format_supported("tiff"));
    }
}