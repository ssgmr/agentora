//! WorldSeed世界生成器

use crate::seed::WorldSeed;
use crate::types::{Position, TerrainType, ResourceType};
use crate::world::{World, region::Region, resource::ResourceNode};
use crate::agent::Agent;
use crate::types::AgentId;
use rand::{Rng, SeedableRng, seq::SliceRandom};

/// 世界生成器
pub struct WorldGenerator;

/// 地形噪声阈值（正弦波叠加输出 [0, 1]）
/// 目标分布：plains 40% / forest 25% / desert 15% / mountain 10% / water 10%
const TERRAIN_THRESHOLDS: &[(f64, TerrainType)] = &[
    (0.28, TerrainType::Water),      // <0.28 → 水域 (~10%)
    (0.45, TerrainType::Plains),     // 0.28-0.45 → 平原 (~15%)
    (0.65, TerrainType::Forest),     // 0.45-0.65 → 森林 (~20%)
    (0.82, TerrainType::Mountain),   // 0.65-0.82 → 山地 (~15%)
    // >0.82 → 沙漠 (~40%，但实际会被阈值截断)
];

/// 低频正弦波产生大区块地形噪声 [0, 1]
/// 不用多层叠加，只用极低频单层正弦，确保区块足够大
fn fractal_noise(x: f64, y: f64, _octaves: usize, seed: u64) -> f64 {
    let s = seed as f64;
    // 极低频正弦波叠加，256地图约1-2个周期
    let v = (x * 0.008 + y * 0.006 + s * 0.5).sin() * 0.4
          + (y * 0.010 - x * 0.004 + s * 0.3).sin() * 0.3
          + ((x + y) * 0.007 + s * 0.7).cos() * 0.3;
    // 归一化到 [0, 1]
    (v * 0.5 + 0.5).clamp(0.0, 1.0)
}

impl WorldGenerator {
    /// 从WorldSeed生成世界
    pub fn generate(seed: &WorldSeed) -> World {
        let mut world = World::new(seed);

        // 生成地形
        Self::generate_terrain(&mut world, seed);

        // 生成区域
        Self::generate_regions(&mut world, seed);

        // 生成资源节点
        Self::generate_resources(&mut world, seed);

        // 生成初始Agent
        Self::generate_agents(&mut world, seed);

        world
    }

    /// 使用 Value Noise + 双线性插值生成大区块地形
    /// 在粗网格(50x50)上随机赋值，平滑插值产生连续地形
    fn generate_terrain(world: &mut World, seed: &WorldSeed) {
        let (width, height) = world.map.size();

        // 粗网格间距：50像素 → 256地图约5x5个网格点，确保区块足够大
        let grid_size = 50.0;

        // 生成网格随机值 [0, 1]
        let grid_w = (width as f64 / grid_size).ceil() as usize + 2;
        let grid_h = (height as f64 / grid_size).ceil() as usize + 2;
        let seed_val = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .wrapping_add(seed.initial_agents as u64);
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed_val);
        let mut grid: Vec<Vec<f64>> = vec![vec![0.0; grid_h]; grid_w];
        for gx in 0..grid_w {
            for gy in 0..grid_h {
                grid[gx][gy] = rng.gen::<f64>();
            }
        }

        // 对每个像素点双线性插值
        for y in 0..height {
            for x in 0..width {
                let fx = x as f64 / grid_size;
                let fy = y as f64 / grid_size;

                let gx0 = fx.floor() as usize;
                let gy0 = fy.floor() as usize;
                let gx1 = gx0 + 1;
                let gy1 = gy0 + 1;

                let sx = fx - gx0 as f64; // 平滑插值因子
                let sy = fy - gy0 as f64;

                // Smoothstep 让过渡更自然
                let sx_s = sx * sx * (3.0 - 2.0 * sx);
                let sy_s = sy * sy * (3.0 - 2.0 * sy);

                // 双线性插值
                let v00 = grid[gx0.min(grid_w - 1)][gy0.min(grid_h - 1)];
                let v10 = grid[gx1.min(grid_w - 1)][gy0.min(grid_h - 1)];
                let v01 = grid[gx0.min(grid_w - 1)][gy1.min(grid_h - 1)];
                let v11 = grid[gx1.min(grid_w - 1)][gy1.min(grid_h - 1)];

                let v0 = v00 * (1.0 - sx_s) + v10 * sx_s;
                let v1 = v01 * (1.0 - sx_s) + v11 * sx_s;
                let noise_value = v0 * (1.0 - sy_s) + v1 * sy_s;

                let terrain = Self::noise_to_terrain(noise_value, &seed.terrain_ratio);
                world.map.set_terrain(Position::new(x, y), terrain);
            }
        }
    }

    /// 将噪声值 [0, 1] 映射到地形类型
    fn noise_to_terrain(noise_value: f64, _ratios: &std::collections::BTreeMap<String, f32>) -> TerrainType {
        for (threshold, terrain) in TERRAIN_THRESHOLDS {
            if noise_value < *threshold {
                return *terrain;
            }
        }
        TerrainType::Desert  // 最高值 → 沙漠
    }

    /// 生成区域划分
    fn generate_regions(world: &mut World, seed: &WorldSeed) {
        let (width, height) = world.map.size();
        let region_size = seed.region_size;

        let region_count_x = width / region_size;
        let region_count_y = height / region_size;

        for ry in 0..region_count_y {
            for rx in 0..region_count_x {
                let id = Region::position_to_region_id(rx * region_size, ry * region_size, region_size);
                let region = Region::new(
                    id,
                    rx * region_size + region_size / 2,
                    ry * region_size + region_size / 2,
                    region_size,
                );
                world.regions.insert(id, region);
            }
        }
    }

    /// 生成资源节点（正弦波聚类分布 + 地形匹配）
    fn generate_resources(world: &mut World, seed: &WorldSeed) {
        let (width, height) = world.map.size();
        let mut rng = rand::thread_rng();

        let resource_seed: u64 = rng.gen();
        let resource_threshold = 0.2; // 噪声高于此值的位置生成资源

        let resource_count = ((width * height) as f32 * seed.resource_density) as usize;
        let resource_types = [ResourceType::Iron, ResourceType::Food, ResourceType::Wood, ResourceType::Water, ResourceType::Stone];

        // 先收集所有候选位置，再均匀抽样到目标数量
        let mut candidates: Vec<(u32, u32)> = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let pos = Position::new(x, y);
                let terrain = world.map.get_terrain(pos);
                if !terrain.is_passable() {
                    continue;
                }
                // 用两层正弦波叠加，产生不规则矿脉
                let n1 = fractal_noise(x as f64, y as f64, 2, resource_seed);
                let n2 = fractal_noise(x as f64, y as f64, 1, resource_seed ^ 0xFF);
                if n1 * 0.7 + n2 * 0.3 > resource_threshold {
                    candidates.push((x, y));
                }
            }
        }

        // 随机抽样到目标数量
        candidates.shuffle(&mut rng);
        let count = resource_count.min(candidates.len());

        for i in 0..count {
            let (x, y) = candidates[i];
            let pos = Position::new(x, y);
            let terrain = world.map.get_terrain(pos);

            // 90% 按地形匹配，10% 随机分布
            let resource_type = if rng.gen::<f32>() < 0.9 {
                Self::terrain_match_resource(terrain, &mut rng)
            } else {
                resource_types[rng.gen_range(0..resource_types.len())]
            };

            let node = ResourceNode::new(pos, resource_type, rng.gen_range(50..200));
            world.resources.insert(pos, node);
        }
    }

    /// 根据地形匹配资源类型
    fn terrain_match_resource(terrain: TerrainType, rng: &mut impl Rng) -> ResourceType {
        match terrain {
            TerrainType::Forest => ResourceType::Wood,
            TerrainType::Mountain => {
                // 山地：50% 铁，50% 石头
                if rng.gen::<f32>() < 0.5 { ResourceType::Iron } else { ResourceType::Stone }
            }
            TerrainType::Plains => ResourceType::Food,
            TerrainType::Water => {
                // 水域边缘：食物（水生植物/鱼类）
                ResourceType::Food
            }
            TerrainType::Desert => {
                // 沙漠：石头（风化岩石）或少量铁矿
                if rng.gen::<f32>() < 0.7 { ResourceType::Stone } else { ResourceType::Iron }
            }
        }
    }

    /// 生成初始Agent
    fn generate_agents(world: &mut World, seed: &WorldSeed) {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let (width, height) = world.map.size();

        for i in 0..seed.initial_agents {
            // 找一个可通行位置
            let mut pos;
            loop {
                let x = rng.gen_range(0..width);
                let y = rng.gen_range(0..height);
                pos = Position::new(x, y);
                if world.map.get_terrain(pos).is_passable() {
                    break;
                }
            }

            let agent = Agent::new(AgentId::new(uuid::Uuid::new_v4().to_string()), format!("Agent_{}", i + 1), pos);

            world.insert_agent_at(agent.id.clone(), agent);
        }
    }
}