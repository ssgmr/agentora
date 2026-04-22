//! 生存系统：进食与饮水

impl crate::agent::Agent {
    /// 进食：饱食度+30，food-1
    /// 返回：(是否成功, 饱食度变化量, 饱食度前值, 饱食度后值, 剩余food数量)
    pub fn eat_food(&mut self) -> (bool, u32, u32, u32, u32) {
        let food = self.inventory.get("food").copied().unwrap_or(0);
        if food == 0 {
            return (false, 0, self.satiety, self.satiety, 0);
        }
        let satiety_before = self.satiety;
        self.inventory.insert("food".to_string(), food - 1);
        if food == 1 {
            self.inventory.remove("food");
        }
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
        self.inventory.insert("water".to_string(), water - 1);
        if water == 1 {
            self.inventory.remove("water");
        }
        self.hydration = (self.hydration + 25).min(100);
        (true, self.hydration - hydration_before, hydration_before, self.hydration, water - 1)
    }
}