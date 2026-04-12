//! WorldSeed世界生成器

use crate::seed::WorldSeed;
use crate::types::{Position, TerrainType, ResourceType};
use crate::world::{World, map::CellGrid, region::Region, resource::ResourceNode};
use crate::agent::Agent;
use crate::types::AgentId;

/// 世界生成器
pub struct WorldGenerator;

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

    /// 生成地形（简单随机分布）
    fn generate_terrain(world: &mut World, seed: &WorldSeed) {
        let (width, height) = world.map.size();
        use rand::Rng;
        let mut rng = rand::thread_rng();

        for y in 0..height {
            for x in 0..width {
                let terrain = Self::random_terrain(&mut rng, &seed.terrain_ratio);
                world.map.set_terrain(Position::new(x, y), terrain);
            }
        }
    }

    /// 随机选择地形
    fn random_terrain(rng: &mut impl rand::Rng, ratios: &std::collections::HashMap<String, f32>) -> TerrainType {
        let total: f32 = ratios.values().sum();
        let roll = rng.gen::<f32>() * total;
        let mut accumulated = 0.0;

        for (name, ratio) in ratios {
            accumulated += ratio;
            if roll < accumulated {
                return Self::terrain_from_name(name);
            }
        }
        TerrainType::Plains
    }

    fn terrain_from_name(name: &str) -> TerrainType {
        match name {
            "plains" => TerrainType::Plains,
            "forest" => TerrainType::Forest,
            "mountain" => TerrainType::Mountain,
            "water" => TerrainType::Water,
            "desert" => TerrainType::Desert,
            _ => TerrainType::Plains,
        }
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

    /// 生成资源节点
    fn generate_resources(world: &mut World, seed: &WorldSeed) {
        let (width, height) = world.map.size();
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let resource_count = (width * height * seed.resource_density as u32) as usize;
        let resource_types = [ResourceType::Iron, ResourceType::Food, ResourceType::Wood, ResourceType::Water, ResourceType::Stone];

        for _ in 0..resource_count {
            let x = rng.gen_range(0..width);
            let y = rng.gen_range(0..height);
            let pos = Position::new(x, y);

            // 只在可通行地形放置资源
            if world.map.get_terrain(pos).is_passable() {
                let resource_type = resource_types[rng.gen_range(0..resource_types.len())];
                let node = ResourceNode::new(pos, resource_type, rng.gen_range(50..200));
                world.resources.insert(pos, node);
            }
        }
    }

    /// 生成初始Agent
    fn generate_agents(world: &mut World, seed: &WorldSeed) {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let (width, height) = world.map.size();

        let templates: Vec<&[f32; 6]> = seed.motivation_templates.values().collect();
        let template_names: Vec<&str> = seed.motivation_templates.keys().map(|s| s.as_str()).collect();

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

            let template_idx = rng.gen_range(0..templates.len().max(1));
            let name = format!("{}_{}", template_names.get(template_idx).unwrap_or(&"Agent"), i + 1);

            let mut agent = Agent::new(AgentId::new(uuid::Uuid::new_v4().to_string()), name, pos);

            // 应用动机模板
            if let Some(template) = templates.get(template_idx) {
                agent.motivation = crate::motivation::MotivationVector::from_array(**template);
            }

            world.insert_agent_at(agent.id.clone(), agent);
        }
    }
}