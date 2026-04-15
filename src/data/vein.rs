use super::enums::VeinType;
use serde::{Serialize, Serializer};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Vein {
    pub vein_type: VeinType,
    pub min_group: i32,
    pub max_group: i32,
    pub min_patch: i32,
    pub max_patch: i32,
    pub min_amount: i32, // times 4e-5 for oil
    pub max_amount: i32,
}

impl Default for Vein {
    fn default() -> Self {
        Self {
            vein_type: VeinType::None,
            min_group: 0,
            max_group: 0,
            min_patch: 0,
            max_patch: 0,
            min_amount: 0,
            max_amount: 0,
        }
    }
}

impl Vein {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn min(&self) -> i32 {
        self.min_group * self.min_amount * self.min_patch
    }
    pub fn max(&self) -> i32 {
        self.max_group * self.max_amount * self.max_patch
    }
    pub fn estimate(&self) -> i64 {
        (self.min_group + self.max_group) as i64 *
        (self.min_amount + self.max_amount) as i64 *
        (self.min_patch + self.max_patch) as i64 / 8i64
    }
}