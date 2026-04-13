use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[repr(i32)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Deserialize, Serialize)]
pub enum StarType {
    MainSeqStar,
    GiantStar,
    WhiteDwarf,
    NeutronStar,
    BlackHole,
}

impl Default for StarType {
    fn default() -> Self {
        Self::MainSeqStar
    }
}
impl StarType {
    pub fn to_num(self) -> i32 {
        match self {
            StarType::MainSeqStar => 1,
            StarType::GiantStar => 2,
            StarType::WhiteDwarf => 3,
            StarType::NeutronStar => 4,
            StarType::BlackHole => 5
        }
    }
}
#[allow(dead_code)]
#[repr(i32)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Deserialize, Serialize)]
pub enum SpectrType {
    M = -4,
    K = -3,
    G = -2,
    F = -1,
    A = 0,
    B = 1,
    O = 2,
    X = 3,
}

#[allow(dead_code)]
#[repr(i32)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Deserialize, Serialize)]
pub enum PlanetType {
    None,
    Vocano,
    Ocean,
    Desert,
    Ice,
    Gas,
}

impl Default for PlanetType {
    fn default() -> Self {
        Self::None
    }
}

#[allow(dead_code)]
#[repr(i32)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Deserialize, Serialize)]
pub enum ThemeDistribute {
    Default,
    Birth,
    Interstellar,
    Rare,
}

impl Default for ThemeDistribute {
    fn default() -> Self {
        Self::Default
    }
}

#[allow(dead_code)]
#[repr(i32)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Deserialize, Serialize)]
pub enum VeinType {
    None,
    Iron,
    Copper,
    Silicium,
    Titanium,
    Stone,
    Coal,
    Oil,
    Fireice,
    Diamond,
    Fractal,
    Crysrub,
    Grat,
    Bamboo,
    Mag,
    Max,
}

impl Default for VeinType {
    fn default() -> Self {
        Self::None
    }
}

impl VeinType {
    pub fn is_rare(&self) -> bool {
        match self {
            VeinType::Fireice
            | VeinType::Diamond
            | VeinType::Fractal
            | VeinType::Crysrub
            | VeinType::Grat
            | VeinType::Bamboo => true,
            _ => false,
        }
    }
}

pub const ORES: [VeinType; 16] = [
    VeinType::None, VeinType::Iron, VeinType::Copper, VeinType::Silicium,
    VeinType::Titanium, VeinType::Stone, VeinType::Coal, VeinType::Oil,
    VeinType::Fireice, VeinType::Diamond, VeinType::Fractal, VeinType::Crysrub,
    VeinType::Grat, VeinType::Bamboo, VeinType::Mag, VeinType::Max,
];