#[repr(i32)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum StarType {
    MainSeqStar,
    GiantStar,
    WhiteDwarf,
    NeutronStar,
    BlackHole,
}

#[repr(i32)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
#[allow(dead_code)]
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
impl From<i32> for SpectrType {
    fn from(i: i32) -> Self {
        match i {
            -4 => SpectrType::M,
            -3 => SpectrType::K,
            -2 => SpectrType::G,
            -1 => SpectrType::F,
            0 => SpectrType::A,
            1 => SpectrType::B,
            2 => SpectrType::O,
            3 => SpectrType::X,
            _ => unreachable!(), // or return a default
        }
    }
}
#[repr(i32)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum PlanetType {
    Volcano,
    Ocean,
    Desert,
    Ice,
    Gas,
}



#[repr(i32)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ThemeDistribute {
    Default,
    Birth,
    Interstellar
}

impl Default for ThemeDistribute {
    fn default() -> Self {
        Self::Default
    }
}

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