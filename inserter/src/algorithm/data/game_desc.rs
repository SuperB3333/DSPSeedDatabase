#[derive(Debug, Clone, Copy)]
pub struct GameDesc {
    pub star_count: usize,
    pub resource_multiplier: f32,
    pub hive_initial_colonize: f64,
    pub hive_max_density: f64,
    pub use_actual_veins: bool,
}

impl GameDesc {

    pub fn is_infinite_resource(&self) -> bool {
        self.resource_multiplier >= 99.5
    }

    pub fn is_rare_resource(&self) -> bool {
        self.resource_multiplier <= 0.1001
    }

    pub fn oil_amount_multiplier(&self) -> f32 {
        if self.is_rare_resource() {
            0.5
        } else {
            1.0
        }
    }

    pub fn gas_coef(&self) -> f32 {
        if self.is_rare_resource() {
            0.8
        } else {
            1.0
        }
    }
}
impl Default for GameDesc {
    fn default() -> Self {
        Self {
            star_count: 64,
            resource_multiplier: 1.0,
            hive_initial_colonize: 1.0,
            hive_max_density: 1.0,
            use_actual_veins: false
        }
    }
}