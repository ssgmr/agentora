//! 下载并测试本地 LLM 推理（集成测试）
//!
//! 需要网络环境，手动触发: `cargo test --test test_llm_inference -- --ignored`

use agentora_ai::{
    LlamaProvider, LlmProvider, LlmRequest, LlmResponse,
    detect_best_backend, ModelDownloader, get_available_models, DownloadProgress,
};
use tokio::sync::mpsc;
use std::io::Write;

/// 下载模型（如果不存在）
async fn ensure_model() -> Result<String, Box<dyn std::error::Error>> {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..").join("..");
    let model_dir = project_root.join("client").join("models");
    let model_path = model_dir.join("Qwen3.5-2B-Q4_K_M.gguf");

    if model_path.exists() {
        let size = std::fs::metadata(&model_path)?.len();
        println!("[1/3] 模型已存在: {:.1} MB", size as f64 / 1_048_576.0);
        return Ok(model_path.to_string_lossy().to_string());
    }

    println!("[1/3] 模型不存在，开始下载...");

    if !model_dir.exists() {
        tokio::fs::create_dir_all(&model_dir).await?;
    }

    let downloader = ModelDownloader::new();
    let (progress_tx, mut progress_rx) = mpsc::channel::<DownloadProgress>(100);

    // 进度显示
    let progress_handle = tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            print!("\r  进度: {:.1} / {:.1} MB ({:.1}%, {:.2} MB/s)",
                progress.downloaded_mb, progress.total_mb,
                progress.percent, progress.speed_mbps);
            let _ = std::io::stdout().flush();
        }
    });

    let models = get_available_models();
    let qwen_model = models.iter().find(|m| m.name.contains("Qwen3.5"))
        .expect("Qwen3.5 模型配置不存在");

    match downloader.download_with_fallback(qwen_model, &model_dir, progress_tx).await {
        Ok(path) => {
            println!("\n  下载完成: {:?}", path);
            Ok(path.to_string_lossy().to_string())
        }
        Err(e) => {
            println!("\n  下载失败: {}", e);
            Err(Box::new(e))
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_download_and_run_llm() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    let model_path = ensure_model().await.expect("模型下载/查找失败");

    println!("\n[2/3] 创建 LlamaProvider...");
    let backend = detect_best_backend();
    println!("  GPU 后端: {}", backend.name());

    let provider = LlamaProvider::new(model_path.clone())
        .expect("LlamaProvider 创建失败");

    println!("  Provider name: {}", provider.name());
    println!("  Provider available: {}", provider.is_available());
    println!("  Provider really_available: {}", provider.is_really_available());

    if !provider.is_really_available() {
        if let Some(err) = provider.get_load_error() {
            println!("  加载错误: {}", err);
        }
        panic!("模型未实际加载");
    }

    println!("\n[3/3] 测试推理...");

    let request = LlmRequest {
        prompt: "你好，请用一句话介绍你自己。".to_string(),
        max_tokens: 256,
        temperature: 0.7,
        response_format: agentora_ai::ResponseFormat::Text,
        stop_sequences: vec!["\n\n".to_string()],
    };

    let start = std::time::Instant::now();
    let response: LlmResponse = provider.generate(request).await
        .expect("推理失败");
    let elapsed = start.elapsed();

    println!("\n  推理耗时: {:?}", elapsed);
    println!("  Token 统计: prompt={}, completion={}, total={}",
        response.usage.prompt_tokens,
        response.usage.completion_tokens,
        response.usage.total_tokens);
    println!("\n  生成内容:\n  {}", response.raw_text.replace('\n', "\n  "));

    assert!(!response.raw_text.is_empty(), "生成内容为空");
    assert!(response.usage.prompt_tokens > 0, "prompt_tokens 应为正数");
    assert!(response.usage.completion_tokens > 0, "completion_tokens 应为正数");
    assert_eq!(response.provider_name, "llama_local");

    println!("\n  测试通过！");
}
