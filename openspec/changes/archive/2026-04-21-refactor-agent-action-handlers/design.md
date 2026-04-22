# 设计：Agent Action Handlers 职责边界

## 架构概览

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         World职责                                            │
│                                                                              │
│   1. 世界级校验（跨Agent协调）                                                 │
│      - 边界检查：目标位置是否在地图内                                          │
│      - 距离限制：Attack/Trade是否相邻                                         │
│      - 存在性校验：目标Agent是否存在/存活                                      │
│      - 资源节点校验：当前位置是否有资源                                        │
│                                                                              │
│   2. 协调执行                                                                 │
│      - 维护pending_trades队列                                                │
│      - 维护agent_positions反向索引                                           │
│      - 分段借用多个Agent，依次调用方法                                         │
│                                                                              │
│   3. 世界状态更新                                                             │
│      - 资源节点current_amount减少                                             │
│      - 建筑建造/拆除                                                          │
│                                                                              │
│   4. 叙事记录                                                                 │
│      - tick_events: Vec<NarrativeEvent>                                      │
│                                                                              │
│   World不直接改Agent内部属性！                                                 │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ 调用
                                    ↓
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Agent职责                                            │
│                                                                              │
│   提供原子级自身状态变更方法：                                                  │
│   - move_to(target)                                                          │
│   - gather/consume (已有)                                                    │
│   - eat_food/drink_water                                                     │
│   - receive_attack/initiate_attack                                           │
│   - freeze_resources/unfreeze_*                                              │
│   - talk_with                                                                │
│   - accept_alliance/reject_alliance (已有)                                   │
│                                                                              │
│   Agent方法只修改自己！不访问World状态！                                        │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Agent 结构体变更

```rust
pub struct Agent {
    // ...现有字段...
    
    /// 冻结的资源（用于待处理交易）
    pub frozen_inventory: HashMap<String, u32>,
    
    /// 当前发起的交易ID（用于取消时解冻）
    pub pending_trade_id: Option<String>,
}
```

## 新增方法签名

### agent/movement.rs

```rust
impl Agent {
    /// 移动到目标位置
    /// 返回：(是否成功, 旧位置, 新位置)
    pub fn move_to(&mut self, target: Position) -> (bool, Position, Position) {
        let old_pos = self.position;
        self.last_position = Some(old_pos);
        self.position = target;
        (true, old_pos, target)
    }
}
```

### agent/survival.rs

```rust
impl Agent {
    /// 进食：饱食度+30，food-1
    /// 返回：(是否成功, 饱食度变化量, 饱食度前值, 饱食度后值, 剩余food数量)
    pub fn eat_food(&mut self) -> (bool, u32, u32, u32, u32) {
        let food = self.inventory.get("food").copied().unwrap_or(0);
        if food == 0 {
            return (false, 0, self.satiety, self.satiety, 0);
        }
        let satiety_before = self.satiety;
        self.inventory.insert("food", food - 1);
        if food == 1 { self.inventory.remove("food"); }
        self.satiety = (self.satiety + 30).min(100);
        (true, self.satiety - satiety_before, satiety_before, self.satiety, food - 1)
    }

    /// 饮水：水分度+25，water-1
    /// 返回：(是否成功, 水分度变化量, 水分度前值, 水分度后值, 剩余water数量)
    pub fn drink_water(&mut self) -> (bool, u32, u32, u32, u32) {
        let water = self.inventory.get("water").copied().unwrap_or(0);
        if water == 0 {
            return (false, 0, self.hydration, self.hydration, 0);
        }
        let hydration_before = self.hydration;
        self.inventory.insert("water", water - 1);
        if water == 1 { self.inventory.remove("water"); }
        self.hydration = (self.hydration + 25).min(100);
        (true, self.hydration - hydration_before, hydration_before, self.hydration, water - 1)
    }
}
```

### agent/combat.rs（重构）

```rust
impl Agent {
    /// 承受攻击：HP减少，记录攻击者为敌人
    /// 参数：damage 由 World 计算（base_damage * terrain_multiplier）
    pub fn receive_attack(&mut self, damage: u32, attacker_id: &AgentId) {
        self.health = self.health.saturating_sub(damage);
        self.relations.insert(attacker_id.clone(), Relation {
            trust: 0.0,
            relation_type: RelationType::Enemy,
            last_interaction_tick: 0,
        });
    }
    
    /// 发起攻击：记录目标为敌人
    pub fn initiate_attack(&mut self, target_id: &AgentId) {
        self.relations.insert(target_id.clone(), Relation {
            trust: 0.0,
            relation_type: RelationType::Enemy,
            last_interaction_tick: 0,
        });
    }
}
```

### agent/trade.rs（重构）

```rust
impl Agent {
    /// 冻结资源：发起交易时，offer资源移到frozen
    /// 返回是否成功（资源不足时失败）
    pub fn freeze_resources(&mut self, offer: HashMap<ResourceType, u32>, trade_id: &str) -> bool {
        // 检查资源足够
        for (resource, amount) in &offer {
            let key = resource.as_str();
            let current = self.inventory.get(key).copied().unwrap_or(0);
            if current < *amount {
                return false;
            }
        }
        // 冻结：从inventory移到frozen_inventory
        for (resource, amount) in &offer {
            let key = resource.as_str();
            let current = self.inventory.get(key).copied().unwrap_or(0);
            self.inventory.insert(key.to_string(), current - amount);
            let frozen = self.frozen_inventory.get(key).copied().unwrap_or(0);
            self.frozen_inventory.insert(key.to_string(), frozen + amount);
        }
        self.pending_trade_id = Some(trade_id.to_string());
        true
    }
    
    /// 完成交易发送方：解冻并实际扣减offer，接收want
    pub fn complete_trade_send(&mut self, offer: HashMap<ResourceType, u32>, want: HashMap<ResourceType, u32>) {
        // offer从frozen移除（实际扣减）
        for (resource, amount) in &offer {
            let key = resource.as_str();
            let frozen = self.frozen_inventory.get(key).copied().unwrap_or(0);
            self.frozen_inventory.insert(key.to_string(), frozen - amount);
        }
        // want加入inventory
        for (resource, amount) in want {
            self.gather(resource, amount);
        }
        self.pending_trade_id = None;
    }
    
    /// 取消交易：解冻资源回到inventory
    pub fn cancel_trade(&mut self, offer: HashMap<ResourceType, u32>) {
        for (resource, amount) in &offer {
            let key = resource.as_str();
            let frozen = self.frozen_inventory.get(key).copied().unwrap_or(0);
            self.frozen_inventory.insert(key.to_string(), frozen - amount);
            let current = self.inventory.get(key).copied().unwrap_or(0);
            self.inventory.insert(key.to_string(), current + amount);
        }
        self.pending_trade_id = None;
    }
    
    /// 接收方交出want资源
    pub fn give_resources(&mut self, want: HashMap<ResourceType, u32>) -> bool {
        for (resource, amount) in &want {
            if !self.consume(*resource, *amount) {
                return false;
            }
        }
        true
    }
    
    /// 接收方获得offer资源
    pub fn receive_resources(&mut self, offer: HashMap<ResourceType, u32>) {
        for (resource, amount) in offer {
            self.gather(resource, amount);
        }
    }
}
```

### agent/social.rs

```rust
impl Agent {
    /// 与附近Agent交谈：记录记忆，增加信任
    pub fn talk_with(&mut self, nearby_ids: &[AgentId], message: &str, tick: u32) {
        for target_id in nearby_ids {
            self.increase_trust(target_id, 2.0);
            self.memory.record(&MemoryEvent {
                tick,
                event_type: "social".to_string(),
                content: format!("与 {} 交流：「{}」", target_id, message),
                emotion_tags: vec!["positive".to_string()],
                importance: 0.5,
            });
        }
    }
    
    /// 被交谈：增加信任，记录记忆
    pub fn receive_talk(&mut self, speaker_id: &AgentId, speaker_name: &str, message: &str, tick: u32) {
        self.increase_trust(speaker_id, 1.0);
        self.memory.record(&MemoryEvent {
            tick,
            event_type: "social".to_string(),
            content: format!("{} 与你交流：「{}」", speaker_name, message),
            emotion_tags: vec!["positive".to_string()],
            importance: 0.5,
        });
    }
}
```

## World Handler 变更示例

### handle_attack（重构后）

```rust
// world/actions.rs
pub fn handle_attack(&mut self, agent_id: &AgentId, target_id: AgentId) -> ActionResult {
    // World校验（保持不变）
    if !self.agents.contains_key(&target_id) {
        return ActionResult::Blocked(...);
    }
    if !self.agents.get(&target_id).map(|a| a.is_alive).unwrap_or(false) {
        return ActionResult::Blocked(...);
    }
    let distance = agent_pos.manhattan_distance(&target_pos);
    if distance > 1 {
        return ActionResult::Blocked(...);
    }
    // 盟友检查...
    
    // World计算damage
    let damage = 10;  // 可扩展为 terrain_multiplier * buff_multiplier
    
    // 分段借用，调用Agent方法
    {
        let target = self.agents.get_mut(&target_id).unwrap();
        target.receive_attack(damage, agent_id);  // Agent方法
    }
    {
        let attacker = self.agents.get_mut(agent_id).unwrap();
        attacker.initiate_attack(&target_id);     // Agent方法
    }
    
    // World维护统计
    self.total_attacks += 1;
    
    // World生成叙事
    self.record_event(...);
    
    ActionResult::SuccessWithDetail(...)
}
```

### handle_trade_accept（重构后）

```rust
pub fn handle_trade_accept(&mut self, agent_id: &AgentId) -> ActionResult {
    // World查找pending_trade
    let trade_idx = self.pending_trades.iter().position(...);
    
    // World校验双方资源足够
    // ...
    
    // 分段借用，调用Agent方法
    {
        let acceptor = self.agents.get_mut(agent_id).unwrap();
        acceptor.give_resources(trade.want.clone());      // Agent方法
        acceptor.receive_resources(trade.offer.clone());  // Agent方法
    }
    {
        let proposer = self.agents.get_mut(&trade.proposer_id).unwrap();
        proposer.complete_trade_send(trade.offer.clone(), trade.want.clone()); // Agent方法
    }
    
    // World移除pending_trade
    self.pending_trades.remove(trade_idx);
    self.total_trades += 1;
    
    // World生成叙事
    self.record_event(...);
}
```

## 文件结构变更

```
agent/
├── mod.rs       # 新增 frozen_inventory, pending_trade_id 字段
├── inventory.rs # 保持不变
├── movement.rs  # 新建：move_to()
├── survival.rs  # 新建：eat_food(), drink_water()
├── combat.rs    # 重构：receive_attack(), initiate_attack()
├── trade.rs     # 重构：freeze_resources(), complete_trade_send(), cancel_trade()
├── social.rs    # 新建：talk_with(), receive_talk()
├── alliance.rs  # 保持不变
```

## Trade 流程图

```
发起方发起 TradeOffer：
┌─────────────────────────────────────┐
│ World.handle_trade_offer            │
│   1. World校验目标存在/存活          │
│   2. World创建PendingTrade          │
│   3. World调用                      │
│      proposer.freeze_resources()    │
│      → offer移到frozen_inventory    │
└─────────────────────────────────────┘

接受方接受 TradeAccept：
┌─────────────────────────────────────┐
│ World.handle_trade_accept           │
│   1. World查找pending_trade         │
│   2. World校验双方资源足够           │
│   3. World分段调用Agent方法：        │
│      acceptor.give_resources(want)  │
│      acceptor.receive_resources(offer)│
│      proposer.complete_trade_send() │
│   4. World移除pending_trade         │
│   5. World记录事件                  │
└─────────────────────────────────────┘

取消/拒绝 TradeReject：
┌─────────────────────────────────────┐
│ World.handle_trade_reject           │
│   1. World查找pending_trade         │
│   2. World调用                      │
│      proposer.cancel_trade(offer)   │
│      → frozen回到inventory          │
│   3. World移除pending_trade         │
└─────────────────────────────────────┘

交易超时自动取消（World.tick_loop）：
┌─────────────────────────────────────┐
│ World 每tick检查pending_trades      │
│   1. 发现超时交易（tick - tick_created │
│      > TRADE_TIMEOUT_TICKS）        │
│   2. World调用                      │
│      proposer.cancel_trade(offer)   │
│      → frozen回到inventory          │
│   3. World移除pending_trade         │
│   4. World记录超时事件              │
└─────────────────────────────────────┘

参数配置：
- TRADE_TIMEOUT_TICKS: 默认 50（约 50 tick 后自动取消）
- 可通过 config/sim.toml 配置 trade_timeout_ticks
```