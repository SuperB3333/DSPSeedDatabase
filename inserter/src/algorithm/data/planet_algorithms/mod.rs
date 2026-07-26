/// Planet algorithm implementations ported from PlanetAlgorithm0 through PlanetAlgorithm14.
/// Each algorithm lazily computes height data for a planet based on its seed and theme.
use super::planet::Planet;

use algo0::PlanetAlgorithm0;
use algo1::PlanetAlgorithm1;
use algo2::PlanetAlgorithm2;
use algo3::PlanetAlgorithm3;
use algo4::PlanetAlgorithm4;
use algo5::PlanetAlgorithm5;
use algo6::PlanetAlgorithm6;
use algo7::PlanetAlgorithm7;
use algo8::PlanetAlgorithm8;
use algo9::PlanetAlgorithm9;
use algo10::PlanetAlgorithm10;
use algo11::PlanetAlgorithm11;
use algo12::PlanetAlgorithm12;
use algo13::PlanetAlgorithm13;

mod algo0;
mod algo1;
mod algo2;
mod algo3;
mod algo4;
mod algo5;
mod algo6;
mod algo7;
mod algo8;
mod algo9;
mod algo10;
mod algo11;
mod algo12;
mod algo13;

#[derive(Clone, Debug, Copy, PartialEq)]
pub enum PlanetAlgorithms {
    A0 = 0,
    A1 = 1,
    A2 = 2,
    A3 = 3,
    A4 = 4,
    A5 = 5,
    A6 = 6,
    A7 = 7,
    A8 = 8,
    A9 = 9,
    A10 = 10,
    A11 = 11,
    A12 = 12,
    A13 = 13,
}

/// Trait for planet algorithms. Each algorithm lazily computes height for individual vertices.
pub trait PlanetAlgorithm {
    /// Compute the height for a single vertex index.
    ///
    /// # Arguments
    /// * `index` - The vertex index (0..planet_raw_data::data_length())
    ///
    /// # Returns
    /// Height in game units (f64).
    fn get_height(&self, index: usize) -> f64;
}

/// Construct the algorithm matching the planet's algo ID.
/// Returns a boxed, fully-initialized algorithm ready for lazy height queries.
pub fn create_and_prepare_algo(planet: &Planet) -> Box<dyn PlanetAlgorithm> {
    match planet.get_algo() {
        PlanetAlgorithms::A0 => Box::new(PlanetAlgorithm0::new(planet)),
        PlanetAlgorithms::A1 => Box::new(PlanetAlgorithm1::new(planet)),
        PlanetAlgorithms::A2 => Box::new(PlanetAlgorithm2::new(planet)),
        PlanetAlgorithms::A3 => Box::new(PlanetAlgorithm3::new(planet)),
        PlanetAlgorithms::A4 => Box::new(PlanetAlgorithm4::new(planet)),
        PlanetAlgorithms::A5 => Box::new(PlanetAlgorithm5::new(planet)),
        PlanetAlgorithms::A6 => Box::new(PlanetAlgorithm6::new(planet)),
        PlanetAlgorithms::A7 => Box::new(PlanetAlgorithm7::new(planet)),
        PlanetAlgorithms::A8 => Box::new(PlanetAlgorithm8::new(planet)),
        PlanetAlgorithms::A9 => Box::new(PlanetAlgorithm9::new(planet)),
        PlanetAlgorithms::A10 => Box::new(PlanetAlgorithm10::new(planet)),
        PlanetAlgorithms::A11 => Box::new(PlanetAlgorithm11::new(planet)),
        PlanetAlgorithms::A12 => Box::new(PlanetAlgorithm12::new(planet)),
        PlanetAlgorithms::A13 => Box::new(PlanetAlgorithm13::new(planet)),
    }
}
