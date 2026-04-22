//! 世界时间推进循环
//!
//! 定期调用 world.advance_tick() 推进世界时间

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

use crate::World;

/// 世界时间推进循环（tick loop）
/// 定期调用 world.tick()，推进世界时间、临时偏好衰减、压力事件触发等
pub async fn run_tick_loop(
    world: Arc<Mutex<World>>,
    is_paused: Arc<AtomicBool>,
    tick_interval_secs: u64,
) {
    tracing::info!("[TickLoop] 启动，间隔={}秒", tick_interval_secs);
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(tick_interval_secs));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        // 暂停时跳过 tick
        if is_paused.load(Ordering::SeqCst) {
            tracing::trace!("[TickLoop] 暂停中，跳过 world.tick()");
            continue;
        }

        // 调用 world.advance_tick() 推进世界时间
        let tick_result = {
            let mut w = world.lock().await;
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                w.advance_tick();
                w.tick
            }))
        };

        match tick_result {
            Ok(tick) => {
                tracing::debug!("[TickLoop] world.tick = {}", tick);
            }
            Err(e) => {
                tracing::error!("[TickLoop] world.tick() panic: {:?}", e);
            }
        }
    }
}

/// 世界时间推进循环（带模式选择）
///
/// P2P 模式下只对本地 Agent 执行生存消耗
pub async fn run_tick_loop_with_mode(
    world: Arc<Mutex<World>>,
    is_paused: Arc<AtomicBool>,
    tick_interval_secs: u64,
    is_p2p_mode: bool,
) {
    tracing::info!("[TickLoop] 启动，间隔={}秒, p2p_mode={}", tick_interval_secs, is_p2p_mode);
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(tick_interval_secs));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        // 暂停时跳过 tick
        if is_paused.load(Ordering::SeqCst) {
            tracing::trace!("[TickLoop] 暂停中，跳过 world.tick()");
            continue;
        }

        // 根据模式选择不同的 advance 方法
        let tick_result = {
            let mut w = world.lock().await;
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if is_p2p_mode {
                    w.advance_tick_local_only();
                } else {
                    w.advance_tick();
                }
                w.tick
            }))
        };

        match tick_result {
            Ok(tick) => {
                tracing::debug!("[TickLoop] world.tick = {}", tick);
            }
            Err(e) => {
                tracing::error!("[TickLoop] world.tick() panic: {:?}", e);
            }
        }

        // P2P 模式下清理过期影子 Agent
        if is_p2p_mode {
            let mut w = world.lock().await;
            let current_tick = w.tick;
            w.cleanup_expired_shadows(current_tick, 300);
        }
    }
}