//! 叙事发射器
//!
//! 从 World 提取叙事事件并发送到 narrative channel。
//! 从 agent_loop.rs 迁移，实现职责单一化。

use crate::world::World;
use crate::snapshot::{NarrativeEvent, NarrativeChannel, AgentSource};
use std::sync::mpsc::Sender;

/// 叙事发射器
pub struct NarrativeEmitter;

impl NarrativeEmitter {
    /// 根据事件类型判定叙事频道
    ///
    /// - 本地专属：Wait（玩家只想看自己的详细思考）
    /// - 附近可见：Move, MoveToward, Gather, Talk, Trade, Attack, Build, Explore, Eat, Drink, Healed
    /// - 世界可见：Milestone, PressureStart, PressureEnd, Death
    pub fn determine_channel(event_type: &str) -> NarrativeChannel {
        match event_type {
            // 世界可见（全局事件）
            "milestone" | "pressure_start" | "pressure_end" | "death" => NarrativeChannel::World,
            // 本地专属（玩家详细思考）
            "wait" => NarrativeChannel::Local,
            // 附近可见（视野内的交互）
            _ => NarrativeChannel::Nearby,
        }
    }

    /// 从 World 提取当前 tick 的叙事事件，并设置正确的频道
    ///
    /// # 参数
    /// - `world`: 世界状态引用
    ///
    /// # 返回
    /// 当前 tick 的叙事事件列表（已设置 channel 和 agent_source）
    pub fn extract(world: &World) -> Vec<NarrativeEvent> {
        world.tick_events.iter().map(|e| {
            let channel = Self::determine_channel(&e.event_type);
            NarrativeEvent {
                tick: e.tick,
                agent_id: e.agent_id.clone(),
                agent_name: e.agent_name.clone(),
                event_type: e.event_type.clone(),
                description: e.description.clone(),
                color_code: e.color_code.clone(),
                channel,
                agent_source: AgentSource::Local,
            }
        }).collect()
    }

    /// 发送叙事事件到 narrative channel
    ///
    /// # 参数
    /// - `narrative_tx`: narrative 发送通道
    /// - `events`: 叙事事件列表
    ///
    /// # 返回
    /// 成功发送的事件数量
    pub fn send_events(
        narrative_tx: &Sender<NarrativeEvent>,
        events: Vec<NarrativeEvent>,
    ) -> usize {
        let mut sent_count = 0;
        for event in events {
            tracing::info!("[Narrative] tick={} channel={} {}: {}",
                event.tick,
                match event.channel {
                    NarrativeChannel::Local => "local",
                    NarrativeChannel::Nearby => "nearby",
                    NarrativeChannel::World => "world",
                },
                event.event_type,
                event.description);
            if let Err(e) = narrative_tx.send(event) {
                tracing::error!("[NarrativeEmitter] narrative 发送失败: {:?}", e);
            } else {
                sent_count += 1;
            }
        }
        sent_count
    }

    /// 提取并发送叙事事件（组合方法）
    ///
    /// # 参数
    /// - `narrative_tx`: narrative 发送通道
    /// - `world`: 世界状态引用
    ///
    /// # 返回
    /// 成功发送的事件数量
    pub fn emit(
        narrative_tx: &Sender<NarrativeEvent>,
        world: &World,
    ) -> usize {
        let events = Self::extract(world);
        Self::send_events(narrative_tx, events)
    }
}