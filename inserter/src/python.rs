#![cfg(feature="python")]
mod algorithm;

use pyo3::prelude::*;




#[pymodule]
mod dsp_generator {
    use pyo3::{IntoPyObjectExt};
    use pyo3::types::*;
    use pyo3::exceptions::*;
    use pyo3::prelude::*;
    use pythonize::pythonize;
    use crate::algorithm::data::game_desc::GameDesc;

    use crate::algorithm::worldgen::galaxy_gen::create_galaxy;

    /// Generates a dsp galaxy and returns a dictionary
    #[pyfunction]
    #[allow(non_snake_case)]
    pub fn generate(py: Python, seed: i32, star_count: usize, resource_multiplier: f32) -> PyResult<Bound<'_, PyAny>> {
        let mut game_desc: GameDesc = GameDesc::default();
        game_desc.seed = seed; game_desc.star_count = star_count; game_desc.resource_multiplier = resource_multiplier;
        let galaxy = create_galaxy(&game_desc);
        pythonize(py, &galaxy.stars).map_err(|e| {
            PyErr::new::<PyRuntimeError, _>(format!("Failed to pythonize galaxy: {}", e))
        })
    }
}