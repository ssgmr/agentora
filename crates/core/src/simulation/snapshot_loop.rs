//! 定期快照生成循环
//!
//! 每5秒生成完整世界快照作为兜底

use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

use crate::{World, WorldSnapshot};

/// 定期 snapshot 兜底循环
pub async fn run_snapshot_loop(
    snapshot_tx: Sender<WorldSnapshot>,
    world: Arc<Mutex<World>>,
    is_paused: Arc<AtomicBool>,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
    interval.tick().await; // 跳过第一次

    loop {
        interval.tick().await;

        // 暂停时跳过 snapshot 发送
        if is_paused.load(Ordering::SeqCst) {
            tracing::trace!("[SnapshotLoop] 暂停中，跳过 snapshot 发送");
            continue;
        }

        let snapshot_opt = {
            let w = world.lock().await;
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| w.snapshot()))
        };

        match snapshot_opt {
            Ok(snapshot) => {
                tracing::info!("[SnapshotLoop] snapshot 生成成功，tick={}", snapshot.tick);
                if snapshot_tx.send(snapshot).is_err() {
                    break;
                }
            }
            Err(e) => {
                tracing::error!("[SnapshotLoop] snapshot() panic: {:?}", e);
                break;
            }
        }
    }
}