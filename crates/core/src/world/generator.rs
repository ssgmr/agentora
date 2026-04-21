//! 世界生成器
//!
//! 从 WorldSeed 生成地形、区域、资源、初始 Agent。

use crate::seed::WorldSeed;
use crate::world::World;
use crate::world::{map, region, resource};
use crate::types::{TerrainType, ResourceType, Position, AgentId, PersonalitySeed};
use crate::agent::Agent;
use std::collections::HashMap;

impl World {
    /// 生成地形（使用 OpenSimplex 噪声 + 百分位映射，确保分布符合配置）
    pub fn generate_terrain(map: &mut map::CellGrid, seed: &WorldSeed) {
        use noise::{OpenSimplex, NoiseFn};

        let (width, height) = map.size();
        let total_cells = width * height;

        // 使用时间戳作为噪声种子
        let base_seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .wrapping_add(seed.initial_agents as u64) as u32;

        // 多个噪声层叠加，产生更丰富的地形
        let noise1 = OpenSimplex::new(base_seed);
        let noise2 = OpenSimplex::new(base_seed.wrapping_add(1000));
        let noise3 = OpenSimplex::new(base_seed.wrapping_add(2000));

        // 先计算所有噪声值
        let mut noise_values: Vec<(u32, u32, f64)> = Vec::with_capacity(total_cells as usize);
        for y in 0..height {
            for x in 0..width {
                // 多层噪声叠加（不同频率产生不同大小的区块）
                let n1 = noise1.get([x as f64 * 0.005, y as f64 * 0.005]);      // 低频大区块
                let n2 = noise2.get([x as f64 * 0.02, y as f64 * 0.02]);        // 中频细节
                let n3 = noise3.get([x as f64 * 0.1, y as f64 * 0.1]);          // 高频微小变化

                // 加权叠加（低频权重高，产生大区块为主）
                let combined = n1 * 0.6 + n2 * 0.3 + n3 * 0.1;
                noise_values.push((x, y, combined));
            }
        }

        // 按噪声值排序（保持空间连续性：相邻像素噪声相近）
        noise_values.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

        // 按配置比例分配地形类型（百分位映射）
        // 配置顺序：water -> plains -> forest -> mountain -> desert
        let terrain_assignment = Self::build_terrain_assignment(total_cells, &seed.terrain_ratio);

        // 统计计数
        let mut plains_count = 0u32;
        let mut forest_count = 0u32;
        let mut mountain_count = 0u32;
        let mut water_count = 0u32;
        let mut desert_count = 0u32;

        // 分配地形
        for (idx, (x, y, _noise)) in noise_values.iter().enumerate() {
            let terrain = terrain_assignment[idx];
            map.set_terrain(Position::new(*x, *y), terrain);

            match terrain {
                TerrainType::Plains => plains_count += 1,
                TerrainType::Forest => forest_count += 1,
                TerrainType::Mountain => mountain_count += 1,
                TerrainType::Water => water_count += 1,
                TerrainType::Desert => desert_count += 1,
            }
        }

        // 输出地形分布统计
        tracing::info!("地形生成完成: {}x{} (百分位映射)", width, height);
        tracing::info!("  plains={} ({:.1}%), forest={} ({:.1}%), mountain={} ({:.1}%), water={} ({:.1}%), desert={} ({:.1}%)",
            plains_count, plains_count as f64 / total_cells as f64 * 100.0,
            forest_count, forest_count as f64 / total_cells as f64 * 100.0,
            mountain_count, mountain_count as f64 / total_cells as f64 * 100.0,
            water_count, water_count as f64 / total_cells as f64 * 100.0,
            desert_count, desert_count as f64 / total_cells as f64 * 100.0);
    }

    /// 按配置比例构建地形分配表（百分位映射）
    pub fn build_terrain_assignment(total_cells: u32, ratios: &std::collections::BTreeMap<String, f32>) -> Vec<TerrainType> {
        let mut assignment: Vec<TerrainType> = Vec::with_capacity(total_cells as usize);

        // 按固定顺序累积分配：water -> plains -> forest -> mountain -> desert
        let terrain_order = [
            ("water", TerrainType::Water),
            ("plains", TerrainType::Plains),
            ("forest", TerrainType::Forest),
            ("mountain", TerrainType::Mountain),
            ("desert", TerrainType::Desert),
        ];

        let mut assigned = 0u32;
        for (name, terrain) in terrain_order {
            let ratio = ratios.get(name).copied().unwrap_or(0.0);
            let count = ((total_cells as f32) * ratio).round() as u32;
            for _ in 0..count {
                assignment.push(terrain);
                assigned += 1;
            }
        }

        // 补齐剩余（因浮点舍入可能少1-2格）
        while assigned < total_cells {
            assignment.push(TerrainType::Plains); // 默认填充平原
            assigned += 1;
        }

        assignment
    }

    /// 生成区域划分
    pub fn generate_regions(regions: &mut HashMap<u32, region::Region>, seed: &WorldSeed) {
        let (width, height) = (seed.map_size[0], seed.map_size[1]);
        let region_size = seed.region_size;

        let region_count_x = width / region_size;
        let region_count_y = height / region_size;

        for ry in 0..region_count_y {
            for rx in 0..region_count_x {
                let id = region::Region::position_to_region_id(rx * region_size, ry * region_size, region_size);
                let region = region::Region::new(
                    id,
                    rx * region_size + region_size / 2,
                    ry * region_size + region_size / 2,
                    region_size,
                );
                regions.insert(id, region);
            }
        }
    }

    /// 生成资源节点（根据地形匹配资源类型）
    pub fn generate_resources(map: &map::CellGrid, resources: &mut HashMap<Position, resource::ResourceNode>, seed: &WorldSeed) {
        use rand::Rng;
        use rand::seq::SliceRandom;
        use noise::{OpenSimplex, NoiseFn};

        let mut rng = rand::thread_rng();
        let (width, height) = map.size();

        // 资源密度噪声（用于聚类分布）
        let resource_seed = rng.gen::<u32>();
        let resource_noise = OpenSimplex::new(resource_seed);
        let resource_freq = 0.02; // 更高频，产生更多资源聚集点

        let target_count = ((width * height) as f32 * seed.resource_density) as usize;
        tracing::info!("generate_resources: map={width}x{height}, density={}, target={}", seed.resource_density, target_count);

        // 先收集所有候选位置（噪声值高于阈值的位置）
        let mut candidates: Vec<(Position, TerrainType)> = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let pos = Position::new(x, y);
                let terrain = map.get_terrain(pos);

                // 只在可通行地形放置资源
                if !terrain.is_passable() {
                    continue;
                }

                // 噪声聚类：归一化后高于阈值的位置成为候选
                let raw_noise = resource_noise.get([x as f64 * resource_freq, y as f64 * resource_freq]);
                let noise_val = (raw_noise / 0.707 + 1.0) / 2.0; // 归一化到 [0, 1]
                if noise_val > 0.35 { // 约65%的可通行区域成为候选
                    candidates.push((pos, terrain));
                }
            }
        }

        // 随机抽样到目标数量
        candidates.shuffle(&mut rng);
        let count = target_count.min(candidates.len());

        // 简单统计（避免 Hash trait）
        let mut wood_count = 0u32;
        let mut food_count = 0u32;
        let mut water_count = 0u32;
        let mut iron_count = 0u32;
        let mut stone_count = 0u32;

        for i in 0..count {
            let (pos, terrain) = candidates[i];

            // 根据地形匹配资源类型（95%匹配，5%随机意外）
            let resource_type = if rng.gen::<f32>() < 0.95 {
                Self::terrain_match_resource(terrain, &mut rng)
            } else {
                // 随机类型作为意外发现
                let random_types = [ResourceType::Iron, ResourceType::Food, ResourceType::Wood, ResourceType::Stone];
                random_types[rng.gen_range(0..random_types.len())]
            };

            // 资源量根据地形调整
            let base_amount = match terrain {
                TerrainType::Forest => rng.gen_range(80..200),  // 森林木材丰富
                TerrainType::Mountain => rng.gen_range(100..250), // 山地矿产丰富
                TerrainType::Plains => rng.gen_range(50..150),   // 平原适中
                TerrainType::Water => rng.gen_range(100..300),   // 水域附近资源丰富
                TerrainType::Desert => rng.gen_range(20..80),    // 沙漠贫瘠
            };

            let node = resource::ResourceNode::new(pos, resource_type, base_amount);
            resources.insert(pos, node);

            // 统计
            match resource_type {
                ResourceType::Wood => wood_count += 1,
                ResourceType::Food => food_count += 1,
                ResourceType::Water => water_count += 1,
                ResourceType::Iron => iron_count += 1,
                ResourceType::Stone => stone_count += 1,
            }
        }

        tracing::info!("资源生成完成: {} 个资源节点 (候选 {} 个)", count, candidates.len());
        tracing::info!("  wood={}, food={}, water={}, iron={}, stone={}", wood_count, food_count, water_count, iron_count, stone_count);
    }

    /// 根据地形匹配资源类型
    pub fn terrain_match_resource(terrain: TerrainType, rng: &mut impl rand::Rng) -> ResourceType {
        match terrain {
            TerrainType::Forest => {
                // 森林：80% 木材，15% 食物（野果），5% 石头
                let roll = rng.gen::<f32>();
                if roll < 0.8 { ResourceType::Wood }
                else if roll < 0.95 { ResourceType::Food }
                else { ResourceType::Stone }
            }
            TerrainType::Mountain => {
                // 山地：50% 铁矿，40% 石头，10% 水（山泉）
                let roll = rng.gen::<f32>();
                if roll < 0.5 { ResourceType::Iron }
                else if roll < 0.9 { ResourceType::Stone }
                else { ResourceType::Water }
            }
            TerrainType::Plains => {
                // 平原：60% 食物，25% 水，10% 石头，5% 铁
                let roll = rng.gen::<f32>();
                if roll < 0.6 { ResourceType::Food }
                else if roll < 0.85 { ResourceType::Water }
                else if roll < 0.95 { ResourceType::Stone }
                else { ResourceType::Iron }
            }
            TerrainType::Water => {
                // 水域边缘：70% 水，30% 食物（水生生物）
                if rng.gen::<f32>() < 0.7 { ResourceType::Water }
                else { ResourceType::Food }
            }
            TerrainType::Desert => {
                // 沙漠：60% 石头，30% 铁（沙漠矿床），10% 水（地下水）
                let roll = rng.gen::<f32>();
                if roll < 0.6 { ResourceType::Stone }
                else if roll < 0.9 { ResourceType::Iron }
                else { ResourceType::Water }
            }
        }
    }

    /// 生成初始 Agent
    pub fn generate_agents(world: &mut World, map_size: (u32, u32), seed: &WorldSeed) {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let (width, height) = map_size;

        for i in 0..seed.initial_agents {
            // 找一个可通行位置（出生在地图中心附近，确保相机能看到）
            let mut pos;
            let cx = width / 2;
            let cy = height / 2;
            let spawn_radius = 16u32; // 中心 32x32 区域内出生
            loop {
                let x = rng.gen_range(cx.saturating_sub(spawn_radius)..(cx + spawn_radius).min(width));
                let y = rng.gen_range(cy.saturating_sub(spawn_radius)..(cy + spawn_radius).min(height));
                pos = Position::new(x, y);
                if world.map.get_terrain(pos).is_passable() {
                    break;
                }
            }

            let name = format!("Agent_{}", i + 1);

            let mut agent = Agent::new(AgentId::new(uuid::Uuid::new_v4().to_string()), name, pos);

            // 任务 2.4：根据性格配置设置 Agent 性格
            let template = seed.agent_personalities.select_template();
            agent.personality = PersonalitySeed::from_template(template);

            tracing::debug!(
                "Agent {} 创建：性格 {} (open={}, agree={}, neuro={})",
                agent.name,
                agent.personality.description,
                agent.personality.openness,
                agent.personality.agreeableness,
                agent.personality.neuroticism
            );

            world.insert_agent_at(agent.id.clone(), agent);
        }
    }
}