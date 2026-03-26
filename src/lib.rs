mod data;
mod worldgen;

use serde::{Deserialize, Serialize};
use pyo3::prelude::*;

#[pymodule]
mod dsp_generator {
    use pyo3::{IntoPyObjectExt, PyClass};
    use pyo3::prelude::*;
    use pythonize::pythonize;
    use crate::data::game_desc::GameDesc;

    use crate::worldgen::galaxy_gen::create_galaxy;



    #[pyfunction]
    #[allow(non_snake_case)]
    fn generate(py: Python, seed: i32, star_count: usize, resource_multiplier: f32) -> PyResult<Py<PyAny>> {
        let mut game_desc: GameDesc = GameDesc::default();
        game_desc.seed = seed; game_desc.star_count = star_count; game_desc.resource_multiplier = resource_multiplier;
        let galaxy = create_galaxy(&game_desc);
        pythonize(py, &galaxy.stars).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to pythonize galaxy: {}", e))
        })?.into_py_any(py)
    }
    #[pyfunction]
    fn find_seed(py: Python, star_count: usize, resource_multiplier: f32, start: i32, end: i32) -> PyResult<Py<PyAny>> {
        let mut game_desc: GameDesc = GameDesc::default();
        game_desc.star_count = star_count; game_desc.resource_multiplier = resource_multiplier;

        pythonize(py, "").map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to pythonize galaxy: {}", e))
        })?.into_py_any(py)
    }

}

struct ScanManager {
    completed_until: i32,
    failed: Vec<Vec<i32>>,
    goal: i32,
    max_job_size: i32
}
impl ScanManager {
    pub fn new(goal: i32, max_job_size: i32) -> Self {
        ScanManager {
            completed_until: 0,
            failed: Vec::new(),
            goal,
            max_job_size
        }
    }
    pub fn get_job(&mut self) -> Vec<i32> {
        if !self.failed.is_empty() {
            self.failed.pop().unwrap();
        }
        let remaining = self.goal - self.completed_until;
        let next_amount = self.max_job_size.min(remaining);
        let start_at = self.completed_until;

        self.completed_until += next_amount;
        vec![start_at, start_at + next_amount]
    }
    pub fn mark_failed(&mut self, start: i32, end: i32) {
        self.failed.push(vec![start, end])
    }
}