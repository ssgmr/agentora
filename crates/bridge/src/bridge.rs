//! SimulationBridge GDExtension 节点
//!
//! Godot 节点定义 + INode 实现 + GDExtension API

use godot::prelude::*;
use godot::classes::{Node, INode};
use std::sync::mpsc::{self, Sender, Receiver};
use std::path::PathBuf;

use agentora_core::simulation::Delta;
use agentora_core::snapshot::NarrativeEvent;
use agentora_core::WorldSnapshot;
use agentora_ai::{load_llm_config, get_available_models, LlmConfig};

// 本地推理 GPU 后端（feature 门控）
#[cfg(feature = "local-inference")]
use agentora_ai::{detect_best_backend, GpuBackend};

use crate::logging::init_logging;
use crate::conversion::{delta_to_dict, snapshot_to_dict};

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
use crate::simulation_runner::{run_simulation_with_api_and_user_config};
use crate::user_config::UserConfig;

/// 加载 llm.toml 配置供 simulation_runner 使用
fn load_llm_config_for_simulation() -> LlmConfig {
    const FALLBACK_LLM_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config/llm.toml");
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let runtime_path = exe_dir.join("config/llm.toml");
            if runtime_path.exists() {
                return load_llm_config(&runtime_path)
                    .ok()
                    .or_else(|| load_llm_config(FALLBACK_LLM_PATH).ok())
                    .unwrap_or_default();
            }
        }
    }
    load_llm_config(FALLBACK_LLM_PATH).unwrap_or_default()
}

/// P2P 事件（从 Simulation 线程发送到 Bridge 主线程）
#[derive(Debug, Clone)]
pub enum P2PEvent {
    PeerConnected { peer_id: String },
    PeerIdReady { peer_id: String },
    StatusChanged { nat_status: String, peer_count: usize, error: String },
}

/// 下载进度事件（从下载线程发送到 Bridge 主线程）
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    /// 下载进度
    Progress {
        model_name: String,
        downloaded_mb: f64,
        total_mb: f64,
        speed_mbps: f64,
    },
    /// 下载完成
    Complete { model_name: String, path: String },
    /// 下载失败
    Failed { model_name: String, error: String },
}

/// 模型加载事件（从加载线程发送到 Bridge 主线程）
#[derive(Debug, Clone)]
pub enum LoadEvent {
    /// 加载开始
    Start {
        model_name: String,
        estimated_time_ms: u32,
    },
    /// 加载进度（估算）
    Progress {
        model_name: String,
        phase: String,
        progress: f64,
    },
    /// 加载完成
    Complete {
        model_name: String,
        backend: String,
        memory_mb: u32,
    },
    /// 加载失败
    Failed { model_name: String, error: String },
}

/// 模拟命令（控制模拟状态）
#[derive(Debug)]
pub enum SimCommand {
    Start,
    Pause,
    SetTickInterval { seconds: f32 },
    InjectPreference {
        agent_id: String,
        key: String,
        boost: f32,
        duration_ticks: u32,
    },
    // P2P 命令
    ConnectToSeed { addr: String },
    QueryPeerInfo {
        query_type: String, // "peers" | "nat_status" | "peer_id"
        response_tx: tokio::sync::oneshot::Sender<String>,
    },
}

/// SimulationBridge GDExtension 节点
#[derive(GodotClass)]
#[class(base=Node)]
pub struct SimulationBridge {
    base: Base<Node>,
    command_sender: Option<Sender<SimCommand>>,
    snapshot_receiver: Option<Receiver<WorldSnapshot>>,
    delta_receiver: Option<Receiver<Delta>>,
    narrative_receiver: Option<Receiver<NarrativeEvent>>,
    /// P2P 事件接收器（从 Simulation 线程发送）
    p2p_event_receiver: Option<Receiver<P2PEvent>>,
    /// 下载进度事件接收器（从下载线程发送）
    download_event_receiver: Option<Receiver<DownloadEvent>>,
    /// 下载进度事件发送器（传递给下载线程）
    download_event_sender: Option<Sender<DownloadEvent>>,
    /// 模型加载事件接收器（从加载线程发送）
    load_event_receiver: Option<Receiver<LoadEvent>>,
    /// 模型加载事件发送器（传递给加载线程）
    load_event_sender: Option<Sender<LoadEvent>>,
    /// 缓存 peer_id（P2P 模式下设置）
    cached_peer_id: String,
    /// 配置文件路径（默认 config/sim.toml）
    #[var]
    config_path: GString,
    current_tick: i64,
    #[var]
    is_paused: bool,
    is_running: bool,
    last_snapshot: Option<WorldSnapshot>,
    #[var]
    selected_agent_id: GString,
    /// 用户配置文件目录
    user_config_dir: PathBuf,
    /// 当前加载的用户配置
    current_user_config: Option<UserConfig>,
}

#[godot_api]
impl INode for SimulationBridge {
    fn init(base: Base<Node>) -> Self {
        // 创建下载进度 channel
        let (download_tx, download_rx) = mpsc::channel::<DownloadEvent>();
        // 创建模型加载 channel
        let (load_tx, load_rx) = mpsc::channel::<LoadEvent>();

        Self {
            base,
            command_sender: None,
            snapshot_receiver: None,
            delta_receiver: None,
            narrative_receiver: None,
            p2p_event_receiver: None,
            download_event_receiver: Some(download_rx),
            download_event_sender: Some(download_tx),
            load_event_receiver: Some(load_rx),
            load_event_sender: Some(load_tx),
            cached_peer_id: String::new(),
            config_path: GString::from("config/sim.toml"),
            current_tick: 0,
            is_paused: false,
            is_running: false,
            last_snapshot: None,
            selected_agent_id: GString::new(),
            user_config_dir: PathBuf::from(resolve_config_path("config")),
            current_user_config: None,
        }
    }

    fn ready(&mut self) {
        init_logging();
        tracing::info!("SimulationBridge: 初始化完成");

        // 检查是否已有用户配置，有则自动启动模拟
        let exists = UserConfig::exists(&self.user_config_dir);

        if exists {
            tracing::info!("SimulationBridge: 检测到已有配置，自动启动模拟");
            let config_path = UserConfig::get_config_path(&self.user_config_dir);
            if let Ok(config) = UserConfig::load(&config_path) {
                self.current_user_config = Some(config.clone());
                self.start_simulation_with_config(config);
            } else {
                tracing::warn!("SimulationBridge: 配置加载失败，等待用户重新配置");
            }
        } else {
            tracing::info!("SimulationBridge: 等待用户配置...");
        }
    }

    fn physics_process(&mut self, _delta: f64) {
        // 1. 优先处理 delta（实时）
        if let Some(receiver) = &self.delta_receiver {
            let mut processed = 0;
            let mut deltas = Vec::new();
            while let Ok(delta) = receiver.try_recv() {
                deltas.push(delta);
                processed += 1;
                if processed >= 100 { break; }
            }
            if !deltas.is_empty() {
                for delta in deltas {
                    let delta_dict = delta_to_dict(&delta);
                    self.base_mut().emit_signal("agent_delta", &[delta_dict.to_variant()]);
                }
            }
        }

        // 2. 处理叙事事件
        if let Some(receiver) = &self.narrative_receiver {
            let mut events = Vec::new();
            while let Ok(event) = receiver.try_recv() {
                events.push(event);
                if events.len() >= 50 { break; }
            }
            for event in events {
                let mut dict: Dictionary<GString, Variant> = Dictionary::new();
                dict.set("tick", &(Variant::from(event.tick as i64)));
                dict.set("agent_id", &event.agent_id.to_variant());
                dict.set("agent_name", &event.agent_name.to_variant());
                dict.set("event_type", &event.event_type.to_variant());
                dict.set("description", &event.description.to_variant());
                dict.set("color", &event.color_code.to_variant());
                self.base_mut().emit_signal("narrative_event", &[dict.to_variant()]);
            }
        }

        // 3. 再处理 snapshot（一致性校验）
        if let Some(receiver) = &self.snapshot_receiver {
            if let Ok(snapshot) = receiver.try_recv() {
                self.current_tick = snapshot.tick as i64;
                self.last_snapshot = Some(snapshot.clone());

                let snapshot_dict = snapshot_to_dict(&snapshot);
                self.base_mut().emit_signal("world_updated", &[snapshot_dict.to_variant()]);
            }
        }

        // 4. 处理 P2P 事件
        if let Some(receiver) = &mut self.p2p_event_receiver {
            let mut events = Vec::new();
            while let Ok(event) = receiver.try_recv() {
                events.push(event);
                if events.len() >= 50 { break; }
            }
            for event in events {
                match event {
                    P2PEvent::PeerConnected { peer_id } => {
                        self.base_mut().emit_signal("peer_connected", &[peer_id.to_variant()]);
                    }
                    P2PEvent::PeerIdReady { peer_id } => {
                        self.cached_peer_id = peer_id;
                    }
                    P2PEvent::StatusChanged { nat_status, peer_count, error } => {
                        let mut dict: Dictionary<Variant, Variant> = Dictionary::new();
                        dict.set("nat_status", nat_status);
                        dict.set("peer_count", &Variant::from(peer_count as i64));
                        dict.set("error", error);
                        self.base_mut().emit_signal("p2p_status_changed", &[dict.to_variant()]);
                    }
                }
            }
        }

        // 5. 处理下载进度事件
        if let Some(receiver) = &mut self.download_event_receiver {
            let mut events = Vec::new();
            while let Ok(event) = receiver.try_recv() {
                events.push(event);
                if events.len() >= 50 { break; }
            }
            for event in events {
                match event {
                    DownloadEvent::Progress { model_name, downloaded_mb, total_mb, speed_mbps } => {
                        self.base_mut().emit_signal("download_progress", &[
                            model_name.to_variant(),
                            downloaded_mb.to_variant(),
                            total_mb.to_variant(),
                            speed_mbps.to_variant(),
                        ]);
                    }
                    DownloadEvent::Complete { model_name, path } => {
                        self.base_mut().emit_signal("model_download_complete", &[
                            path.to_variant(),
                        ]);
                        tracing::info!("模型 {} 下载完成: {}", model_name, path);
                    }
                    DownloadEvent::Failed { model_name, error } => {
                        self.base_mut().emit_signal("model_download_failed", &[
                            error.to_variant(),
                        ]);
                        tracing::error!("模型 {} 下载失败: {}", model_name, error);
                    }
                }
            }
        }

        // 6. 处理模型加载事件
        if let Some(receiver) = &mut self.load_event_receiver {
            let mut events = Vec::new();
            while let Ok(event) = receiver.try_recv() {
                events.push(event);
                if events.len() >= 50 { break; }
            }
            for event in events {
                match event {
                    LoadEvent::Start { model_name, estimated_time_ms } => {
                        self.base_mut().emit_signal("model_load_start", &[
                            model_name.to_variant(),
                            Variant::from(estimated_time_ms as i64),
                        ]);
                        tracing::info!("模型 {} 开始加载，估算时间 {} ms", model_name, estimated_time_ms);
                    }
                    LoadEvent::Progress { model_name, phase, progress } => {
                        self.base_mut().emit_signal("model_load_progress", &[
                            phase.to_variant(),
                            progress.to_variant(),
                            model_name.to_variant(),
                        ]);
                    }
                    LoadEvent::Complete { model_name, backend, memory_mb } => {
                        self.base_mut().emit_signal("model_load_complete", &[
                            model_name.to_variant(),
                            backend.to_variant(),
                            Variant::from(memory_mb as i64),
                        ]);
                        tracing::info!("模型 {} 加载完成，后端: {}, 内存: {} MB", model_name, backend, memory_mb);
                    }
                    LoadEvent::Failed { model_name, error } => {
                        self.base_mut().emit_signal("model_load_failed", &[
                            model_name.to_variant(),
                            error.to_variant(),
                        ]);
                        tracing::error!("模型 {} 加载失败: {}", model_name, error);
                    }
                }
            }
        }
    }
}

#[godot_api]
impl SimulationBridge {
    #[signal]
    fn world_updated(snapshot: Variant);

    #[signal]
    fn agent_delta(delta: Variant);

    #[signal]
    fn agent_selected(agent_id: GString);

    #[signal]
    fn narrative_event(event: Variant);

    #[signal]
    fn peer_connected(peer_id: GString);

    #[signal]
    fn p2p_status_changed(status: Variant);

    // ===== 用户配置信号 =====

    /// 下载进度（含模型名称）
    #[signal]
    fn download_progress(model_name: GString, downloaded_mb: f64, total_mb: f64, speed_mbps: f64);

    #[signal]
    fn model_download_complete(path: GString);

    #[signal]
    fn model_download_failed(error: GString);

    // ===== 模型加载信号 =====

    /// 模型加载开始
    #[signal]
    fn model_load_start(model_name: GString, estimated_time_ms: i64);

    /// 模型加载进度
    #[signal]
    fn model_load_progress(phase: GString, progress: f64, model_name: GString);

    /// 模型加载完成
    #[signal]
    fn model_load_complete(model_name: GString, backend: GString, memory_mb: i64);

    /// 模型加载失败
    #[signal]
    fn model_load_failed(model_name: GString, error: GString);

    // ===== 用户配置 API =====

    /// 检测用户配置是否存在
    #[func]
    fn has_user_config(&self) -> bool {
        UserConfig::exists(&self.user_config_dir)
    }

    /// 获取用户配置
    #[func]
    fn get_user_config(&mut self) -> Dictionary<Variant, Variant> {
        let config_path = UserConfig::get_config_path(&self.user_config_dir);

        // 如果已有缓存，直接返回
        if let Some(config) = &self.current_user_config {
            return self.user_config_to_dict(config);
        }

        // 否则尝试加载
        if let Ok(config) = UserConfig::load(&config_path) {
            self.current_user_config = Some(config.clone());
            return self.user_config_to_dict(&config);
        }

        // 返回默认配置
        let default_config = UserConfig::default();
        self.current_user_config = Some(default_config.clone());
        self.user_config_to_dict(&default_config)
    }

    /// 设置用户配置
    #[func]
    fn set_user_config(&mut self, config_dict: Dictionary<Variant, Variant>) -> bool {
        let config = self.dict_to_user_config(&config_dict);

        // 验证配置
        if let Err(e) = config.validate() {
            godot::global::print(&[Variant::from(format!("配置验证失败: {}", e))]);
            return false;
        }

        // 保存配置
        let config_path = UserConfig::get_config_path(&self.user_config_dir);
        if let Err(e) = config.save(&config_path) {
            godot::global::print(&[Variant::from(format!("配置保存失败: {}", e))]);
            return false;
        }

        self.current_user_config = Some(config.clone());
        godot::global::print(&[Variant::from("用户配置已保存")]);

        // 保存后启动模拟
        self.start_simulation_with_config(config);
        true
    }

    /// 获取可用模型列表
    #[func]
    fn get_available_models(&self) -> Array<Variant> {
        let models = get_available_models();
        let mut arr: Array<Variant> = Array::new();

        for model in models {
            let mut dict: Dictionary<GString, Variant> = Dictionary::new();
            dict.set("name", &model.name.to_variant());
            dict.set("filename", &model.filename.to_variant());
            dict.set("size_mb", &(Variant::from(model.size_mb as i64)));
            dict.set("description", &model.description.to_variant());
            dict.set("primary_url", &model.primary_url.to_variant());
            dict.set("fallback_url", &model.fallback_url.to_variant());
            arr.push(&dict.to_variant());
        }

        arr
    }

    /// 获取已下载的模型列表
    /// 返回值：Array<Dictionary>，每个字典含 name, path, size_mb
    #[func]
    fn get_downloaded_models(&self) -> Array<Variant> {
        let models_dir = PathBuf::from("models");
        let mut arr: Array<Variant> = Array::new();

        if !models_dir.exists() {
            return arr;
        }

        // 扫描所有 .gguf 文件
        if let Ok(entries) = std::fs::read_dir(&models_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("gguf") {
                    let file_name = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let size_mb = if let Ok(metadata) = path.metadata() {
                        metadata.len() as f64 / 1_048_576.0
                    } else {
                        0.0
                    };

                    let mut dict: Dictionary<GString, Variant> = Dictionary::new();
                    dict.set("name", &file_name.to_variant());
                    dict.set("path", &path.to_string_lossy().to_string().to_variant());
                    dict.set("size_mb", &Variant::from(size_mb));
                    arr.push(&dict.to_variant());
                }
            }
        }

        arr
    }

    /// 启动模型下载（异步，进度通过 download_progress 信号反馈）
    #[func]
    fn download_model(&self, model_name: GString, url: GString, dest: GString) -> bool {
        let model_name_str = model_name.to_string();
        let url_str = url.to_string();
        let dest_str = dest.to_string();
        let dest_path = PathBuf::from(&dest_str);

        // 获取 download_event_sender（如果没有则创建新的 channel）
        let sender = match &self.download_event_sender {
            Some(s) => s.clone(),
            None => {
                godot::global::print(&[Variant::from("download_event_sender 未初始化")]);
                return false;
            }
        };

        godot::global::print(&[Variant::from(format!("开始下载模型: {} ({})", model_name_str, url_str))]);

        // 在后台线程运行下载
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                use agentora_ai::{ModelDownloader, DownloadProgress};
                use tokio::sync::mpsc;

                let downloader = ModelDownloader::new();
                let (progress_tx, mut progress_rx) = mpsc::channel::<DownloadProgress>(100);

                // 进度转发线程：将 AI crate 的 DownloadProgress 转换为 Bridge 的 DownloadEvent
                let sender_clone = sender.clone();
                let model_name_for_progress = model_name_str.clone();
                tokio::spawn(async move {
                    while let Some(progress) = progress_rx.recv().await {
                        let event = DownloadEvent::Progress {
                            model_name: model_name_for_progress.clone(),
                            downloaded_mb: progress.downloaded_mb,
                            total_mb: progress.total_mb,
                            speed_mbps: progress.speed_mbps,
                        };
                        if sender_clone.send(event).is_err() {
                            break;
                        }
                    }
                });

                // 执行下载
                let result = downloader.download(&url_str, &dest_path, progress_tx).await;

                // 发送完成/失败事件
                match result {
                    Ok(path) => {
                        let _ = sender.send(DownloadEvent::Complete {
                            model_name: model_name_str,
                            path: path.to_string_lossy().to_string(),
                        });
                    }
                    Err(e) => {
                        let _ = sender.send(DownloadEvent::Failed {
                            model_name: model_name_str,
                            error: e.to_string(),
                        });
                    }
                }
            });
        });

        true
    }

    /// 取消下载（暂未实现）
    #[func]
    fn cancel_download(&self) -> bool {
        // TODO: 实现取消下载
        godot::global::print(&[Variant::from("取消下载功能尚未实现")]);
        false
    }

    // ===== GPU 后端查询 API =====

    /// 获取最优 GPU 后端名称
    /// 返回值: "metal" | "vulkan" | "cuda" | "cpu" | "unavailable"（feature 未启用）
    #[func]
    fn get_gpu_backend(&self) -> GString {
        #[cfg(feature = "local-inference")]
        {
            let backend = detect_best_backend();
            GString::from(backend.name())
        }

        #[cfg(not(feature = "local-inference"))]
        {
            GString::from("unavailable")
        }
    }

    /// 获取 GPU 后端详细信息
    /// 返回值: Dictionary { "name": "...", "is_gpu": bool, "n_gpu_layers": int }
    #[func]
    fn get_gpu_backend_info(&self) -> Dictionary<GString, Variant> {
        let mut dict: Dictionary<GString, Variant> = Dictionary::new();

        #[cfg(feature = "local-inference")]
        {
            let backend = detect_best_backend();
            dict.set("name", &backend.name().to_variant());
            dict.set("is_gpu", &(backend != GpuBackend::Cpu).to_variant());
            dict.set("n_gpu_layers", &(Variant::from(backend.n_gpu_layers() as i64)));
        }

        #[cfg(not(feature = "local-inference"))]
        {
            dict.set("name", &"unavailable".to_variant());
            dict.set("is_gpu", &false.to_variant());
            dict.set("n_gpu_layers", &(Variant::from(0)));
            dict.set("error", &"local-inference feature 未启用".to_variant());
        }

        dict
    }

    // ===== 辅助方法 =====

    /// UserConfig 转换为 Godot Dictionary（扁平结构，与 dict_to_user_config 对应）
    fn user_config_to_dict(&self, config: &UserConfig) -> Dictionary<Variant, Variant> {
        let mut dict: Dictionary<Variant, Variant> = Dictionary::new();

        // LLM 配置（扁平结构）
        dict.set("llm_mode", &config.llm.mode.to_variant());
        dict.set("llm_provider_type", &config.llm.provider_type.to_variant());
        dict.set("llm_api_endpoint", &config.llm.api_endpoint.to_variant());
        dict.set("llm_api_token", &config.llm.api_token.to_variant());
        dict.set("llm_model_name", &config.llm.model_name.to_variant());
        dict.set("llm_local_model_path", &config.llm.local_model_path.to_variant());

        // Agent 配置（扁平结构）
        dict.set("agent_name", &config.agent.name.to_variant());
        dict.set("agent_custom_prompt", &config.agent.custom_prompt.to_variant());
        dict.set("agent_icon_id", &config.agent.icon_id.to_variant());
        dict.set("agent_custom_icon_path", &config.agent.custom_icon_path.to_variant());

        // P2P 配置（扁平结构）
        dict.set("p2p_mode", &config.p2p.mode.to_variant());
        dict.set("p2p_seed_address", &config.p2p.seed_address.to_variant());

        dict
    }

    /// Godot Dictionary 转换为 UserConfig
    fn dict_to_user_config(&self, dict: &Dictionary<Variant, Variant>) -> UserConfig {
        use crate::user_config::{LlmUserConfig, AgentUserConfig, P2PUserConfig};

        // 默认值
        let mut llm = LlmUserConfig {
            mode: "rule_only".to_string(),
            provider_type: "openai".to_string(),
            api_endpoint: String::new(),
            api_token: String::new(),
            model_name: String::new(),
            local_model_path: String::new(),
        };

        let mut agent = AgentUserConfig {
            name: "智行者".to_string(),
            custom_prompt: String::new(),
            icon_id: "default".to_string(),
            custom_icon_path: String::new(),
        };

        let mut p2p = P2PUserConfig {
            mode: "single".to_string(),
            seed_address: String::new(),
        };

        // 解析 LLM 配置（扁平结构）
        if let Some(v) = dict.get("llm_mode") { llm.mode = v.to_string(); }
        if let Some(v) = dict.get("llm_provider_type") { llm.provider_type = v.to_string(); }
        if let Some(v) = dict.get("llm_api_endpoint") { llm.api_endpoint = v.to_string(); }
        if let Some(v) = dict.get("llm_api_token") { llm.api_token = v.to_string(); }
        if let Some(v) = dict.get("llm_model_name") { llm.model_name = v.to_string(); }
        if let Some(v) = dict.get("llm_local_model_path") { llm.local_model_path = v.to_string(); }

        // 解析 Agent 配置（扁平结构）
        if let Some(v) = dict.get("agent_name") { agent.name = v.to_string(); }
        if let Some(v) = dict.get("agent_custom_prompt") { agent.custom_prompt = v.to_string(); }
        if let Some(v) = dict.get("agent_icon_id") { agent.icon_id = v.to_string(); }
        if let Some(v) = dict.get("agent_custom_icon_path") { agent.custom_icon_path = v.to_string(); }

        // 解析 P2P 配置（扁平结构）
        if let Some(v) = dict.get("p2p_mode") { p2p.mode = v.to_string(); }
        if let Some(v) = dict.get("p2p_seed_address") { p2p.seed_address = v.to_string(); }

        UserConfig { llm, agent, p2p }
    }

    #[func]
    fn start_simulation(&mut self) {
        if self.is_running {
            godot::global::print(&[Variant::from("SimulationBridge: 模拟已在运行")]);
            return;
        }
        godot::global::print(&[Variant::from(format!(
            "SimulationBridge: 启动模拟 [config={}]", self.config_path
        ))]);

        let (snapshot_tx, snapshot_rx) = mpsc::channel::<WorldSnapshot>();
        let (delta_tx, delta_rx) = mpsc::channel::<Delta>();
        let (narrative_tx, narrative_rx) = mpsc::channel::<NarrativeEvent>();
        let (cmd_tx, cmd_rx) = mpsc::channel::<SimCommand>();
        let (p2p_event_tx, p2p_event_rx) = mpsc::channel::<P2PEvent>();

        self.snapshot_receiver = Some(snapshot_rx);
        self.delta_receiver = Some(delta_rx);
        self.narrative_receiver = Some(narrative_rx);
        self.p2p_event_receiver = Some(p2p_event_rx);
        self.command_sender = Some(cmd_tx);
        self.is_running = true;
        self.is_paused = false;

        // 加载 UserConfig（如果存在）
        let user_config_path = UserConfig::get_config_path(&PathBuf::from(resolve_config_path("config")));
        let user_config = UserConfig::load(&user_config_path).ok();
        if user_config.is_some() {
            tracing::info!("[Bridge] UserConfig 加载成功: {}", user_config_path.display());
        } else {
            tracing::info!("[Bridge] 无 UserConfig，使用默认配置");
        }

        // 加载 llm.toml 配置（decision/memory 参数 + Provider 默认值）
        let llm_config = load_llm_config_for_simulation();

        let config_path = self.config_path.to_string();

        // 使用 simulation_runner 模块运行模拟（Provider 创建在其中统一处理）
        std::thread::spawn(move || {
            run_simulation_with_api_and_user_config(
                snapshot_tx, delta_tx, narrative_tx, cmd_rx, p2p_event_tx,
                None, llm_config, config_path, user_config
            );
        });

        godot::global::print(&[Variant::from("SimulationBridge: 模拟已启动（事件驱动模式）")]);
    }

    /// 使用已保存的配置启动模拟（内部方法，供 set_user_config 调用）
    fn start_simulation_with_config(&mut self, config: UserConfig) {
        if self.is_running {
            godot::global::print(&[Variant::from("SimulationBridge: 模拟已在运行")]);
            return;
        }
        godot::global::print(&[Variant::from(format!(
            "SimulationBridge: 启动模拟 [config={}]", self.config_path
        ))]);

        let (snapshot_tx, snapshot_rx) = mpsc::channel::<WorldSnapshot>();
        let (delta_tx, delta_rx) = mpsc::channel::<Delta>();
        let (narrative_tx, narrative_rx) = mpsc::channel::<NarrativeEvent>();
        let (cmd_tx, cmd_rx) = mpsc::channel::<SimCommand>();
        let (p2p_event_tx, p2p_event_rx) = mpsc::channel::<P2PEvent>();

        self.snapshot_receiver = Some(snapshot_rx);
        self.delta_receiver = Some(delta_rx);
        self.narrative_receiver = Some(narrative_rx);
        self.p2p_event_receiver = Some(p2p_event_rx);
        self.command_sender = Some(cmd_tx);
        self.is_running = true;
        self.is_paused = false;

        // 加载 llm.toml 配置（decision/memory 参数 + Provider 默认值）
        let llm_config = load_llm_config_for_simulation();

        let config_path = self.config_path.to_string();

        // 使用 simulation_runner 模块运行模拟（Provider 创建在其中统一处理）
        std::thread::spawn(move || {
            run_simulation_with_api_and_user_config(
                snapshot_tx, delta_tx, narrative_tx, cmd_rx, p2p_event_tx,
                None, llm_config, config_path, Some(config)
            );
        });

        godot::global::print(&[Variant::from("SimulationBridge: 模拟已启动（事件驱动模式）")]);
    }

    #[func]
    fn start(&mut self) {
        self.start_simulation();
    }

    #[func]
    fn pause(&mut self) {
        self.toggle_pause();
    }

    #[func]
    fn get_tick(&self) -> i64 {
        self.current_tick
    }

    #[func]
    fn get_agent_count(&self) -> i64 {
        match &self.last_snapshot {
            Some(snapshot) => snapshot.agents.len() as i64,
            None => 5,
        }
    }

    #[func]
    fn toggle_pause(&mut self) {
        self.is_paused = !self.is_paused;
        if let Some(tx) = &self.command_sender {
            let cmd = if self.is_paused {
                SimCommand::Pause
            } else {
                SimCommand::Start
            };
            let _ = tx.send(cmd);
        }
        godot::global::print(&[Variant::from(format!("SimulationBridge: 暂停状态 = {}", self.is_paused))]);
    }

    #[func]
    fn inject_preference(&self, agent_id: String, key: String, boost: f32, duration: i32) {
        if let Some(tx) = &self.command_sender {
            let _ = tx.send(SimCommand::InjectPreference {
                agent_id,
                key,
                boost,
                duration_ticks: duration as u32,
            });
        }
    }

    #[func]
    fn set_tick_interval(&self, seconds: f32) {
        if let Some(tx) = &self.command_sender {
            let _ = tx.send(SimCommand::SetTickInterval { seconds });
        }
    }

    #[func]
    fn get_agent_data(&self, agent_id: String) -> Variant {
        let mut dict: Dictionary<GString, Variant> = Dictionary::new();
        if let Some(snapshot) = &self.last_snapshot {
            if let Some(agent) = snapshot.agents.iter().find(|a| a.id == agent_id) {
                dict.set("id", &agent.id.clone().to_variant());
                dict.set("name", &agent.name.clone().to_variant());
                dict.set("health", &(Variant::from(agent.health as i64)));
                dict.set("max_health", &(Variant::from(agent.max_health as i64)));
                dict.set("satiety", &(Variant::from(agent.satiety as i64)));
                dict.set("hydration", &(Variant::from(agent.hydration as i64)));
                dict.set("is_alive", &agent.is_alive.to_variant());
                dict.set("age", &(Variant::from(agent.age as i64)));
                dict.set("level", &(Variant::from(agent.level as i64)));
                dict.set("current_action", &agent.current_action.clone().to_variant());
                dict.set("action_result", &agent.action_result.clone().to_variant());
                dict.set("reasoning", &agent.reasoning.clone().unwrap_or_default().to_variant());
                let pos = Vector2::new(agent.position.0 as f32, agent.position.1 as f32);
                dict.set("position", &pos.to_variant());
                let mut inv_dict: Dictionary<GString, Variant> = Dictionary::new();
                for (k, v) in &agent.inventory_summary {
                    inv_dict.set(k.as_str(), &Variant::from(*v as i64));
                }
                dict.set("inventory_summary", &inv_dict.to_variant());
            }
        }
        dict.to_variant()
    }

    #[func]
    fn select_agent(&mut self, agent_id: GString) {
        self.selected_agent_id = agent_id.clone();
        self.base_mut().emit_signal("agent_selected", &[agent_id.to_variant()]);
    }

    // ===== P2P API =====

    /// 连接到种子节点
    #[func]
    fn connect_to_seed(&self, addr: GString) -> bool {
        if let Some(tx) = &self.command_sender {
            let _ = tx.send(SimCommand::ConnectToSeed { addr: addr.to_string() });
            true
        } else {
            false
        }
    }

    /// 获取本地 peer_id（P2P 模式下返回缓存值，中心化模式返回空串）
    #[func]
    fn get_peer_id(&self) -> GString {
        GString::from(&self.cached_peer_id)
    }

    /// 获取已连接 peers 列表
    /// 返回值：JSON 字符串，格式 [{"peer_id": "...", "connection_type": "..."}]
    #[func]
    fn get_connected_peers(&self) -> GString {
        // 同步查询：使用 oneshot 通道
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        if let Some(tx) = &self.command_sender {
            let _ = tx.send(SimCommand::QueryPeerInfo {
                query_type: "peers".to_string(),
                response_tx,
            });
            // 使用 blocking_recv 阻塞等待响应
            if let Ok(json_str) = response_rx.blocking_recv() {
                return GString::from(&json_str);
            }
        }
        GString::from("[]")
    }

    /// 获取 NAT 状态
    /// 返回值：JSON 字符串，格式 {"status": "...", "address": "..."}
    #[func]
    fn get_nat_status(&self) -> Dictionary<Variant, Variant> {
        let mut dict: Dictionary<Variant, Variant> = Dictionary::new();
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        if let Some(tx) = &self.command_sender {
            let _ = tx.send(SimCommand::QueryPeerInfo {
                query_type: "nat_status".to_string(),
                response_tx,
            });
            // 使用 blocking_recv 阻塞等待响应
            if let Ok(json_str) = response_rx.blocking_recv() {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    if let Some(status) = val.get("status").and_then(|v| v.as_str()) {
                        dict.set("status", status);
                    }
                    if let Some(addr) = val.get("address").and_then(|v| v.as_str()) {
                        dict.set("address", addr);
                    }
                }
            }
        }
        dict
    }

    /// 获取订阅的 topic 列表（新增）
    /// 返回值：JSON 字符串，格式 ["topic1", "topic2", ...]
    #[func]
    fn get_subscribed_topics(&self) -> GString {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        if let Some(tx) = &self.command_sender {
            let _ = tx.send(SimCommand::QueryPeerInfo {
                query_type: "topics".to_string(),
                response_tx,
            });
            // 使用 blocking_recv 阻塞等待响应
            if let Ok(json_str) = response_rx.blocking_recv() {
                return GString::from(&json_str);
            }
        }
        GString::from("[]")
    }
}