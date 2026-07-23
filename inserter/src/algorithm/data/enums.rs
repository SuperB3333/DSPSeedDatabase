use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[allow(dead_code)]
#[repr(i32)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
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

#[allow(dead_code)]
#[repr(i32)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
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

impl TryFrom<i32> for SpectrType {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            -4 => Ok(SpectrType::M),
            -3 => Ok(SpectrType::K),
            -2 => Ok(SpectrType::G),
            -1 => Ok(SpectrType::F),
            0 => Ok(SpectrType::A),
            1 => Ok(SpectrType::B),
            2 => Ok(SpectrType::O),
            3 => Ok(SpectrType::X),
            _ => Err(value),
        }
    }
}

#[allow(dead_code)]
#[repr(i32)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum PlanetType {
    None,
    Volcano,
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
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
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
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
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
        matches!(
            self,
            VeinType::Fireice
                | VeinType::Diamond
                | VeinType::Fractal
                | VeinType::Crysrub
                | VeinType::Grat
                | VeinType::Bamboo
                | VeinType::Mag
        )
    }
}
pub const ORES: [VeinType; 16] = [
    VeinType::None,
    VeinType::Iron,
    VeinType::Copper,
    VeinType::Silicium,
    VeinType::Titanium,
    VeinType::Stone,
    VeinType::Coal,
    VeinType::Oil,
    VeinType::Fireice,
    VeinType::Diamond,
    VeinType::Fractal,
    VeinType::Crysrub,
    VeinType::Grat,
    VeinType::Bamboo,
    VeinType::Mag,
    VeinType::Max,
];

impl TryFrom<i32> for VeinType {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(VeinType::None),
            1 => Ok(VeinType::Iron),
            2 => Ok(VeinType::Copper),
            3 => Ok(VeinType::Silicium),
            4 => Ok(VeinType::Titanium),
            5 => Ok(VeinType::Stone),
            6 => Ok(VeinType::Coal),
            7 => Ok(VeinType::Oil),
            8 => Ok(VeinType::Fireice),
            9 => Ok(VeinType::Diamond),
            10 => Ok(VeinType::Fractal),
            11 => Ok(VeinType::Crysrub),
            12 => Ok(VeinType::Grat),
            13 => Ok(VeinType::Bamboo),
            14 => Ok(VeinType::Mag),
            15 => Ok(VeinType::Max),
            _ => Err(value),
        }
    }
}

#[repr(i32)]
#[derive(Clone, Debug, serde::Serialize, PartialEq, Copy)]
pub enum OceanType {
    None = 0,
    Ice = -2,
    Lava = -1,
    Water = 1000,
    Acid = 1116
}