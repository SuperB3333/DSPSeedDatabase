mod data;
mod worldgen;

use serde::{Deserialize, Serialize};
use pyo3::prelude::*;
#[pymodule]
mod dsp_generator {
    use std::iter::repeat_n;
    use std::ops::Deref;
    use pyo3::{IntoPyObjectExt};
    use pyo3::types::*;
    use pyo3::exceptions::*;
    use pyo3::prelude::*;
    use pythonize::pythonize;
    use crate::data::enums::{PlanetType, ORES};
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
    fn generate_formatted(py: Python, seed: i32) -> PyResult<Bound<'_, PyTuple>> {
        let mut game_desc: GameDesc = GameDesc::default();
        game_desc.seed = seed;
        let galaxy = create_galaxy(&game_desc);
        let mut stars = vec![];
        let mut planets = vec![];

        for solar_system in galaxy.stars {
            let star = solar_system.star.clone();
            let star_id = star.index as i32 + seed * 100;

            let star_line = (
                star_id,
                seed,
                star.position.magnitude(),
                star.index,
                star.get_luminosity(),
                star.get_dyson_radius(),
                star.star_type.clone() as i32 + 1,
                star.get_spectr().clone() as i32
            );
            stars.push(star_line);

            for planet in solar_system.get_planets() {
                let satellite_count = -1;//todo implement

                let mut gas_h = &0.0; let mut gas_d = &0.0; let mut gas_i = &0.0;
                if planet.get_type() == &PlanetType::Gas {
                    for (gas, rate) in planet.get_gases() {
                        match gas {
                            1120 => gas_h = rate,
                            1121 => gas_d = rate,
                            1011 => gas_i = rate,
                            _ => return Err(PyRuntimeError::new_err("Illegal state: gas was not 1120, 1121 or 1011! This should never happen."))
                        }
                    }
                }
                let planet_line = vec![
                    star_id                                                                 .into_bound_py_any(py)?,
                    planet.index                                                            .into_bound_py_any(py)?,
                    planet.get_theme().water_item_id                                        .into_bound_py_any(py)?,
                    (planet.get_type() == &PlanetType::Gas)                                 .into_bound_py_any(py)?,
                    planet.get_orbital_radius()                                             .into_bound_py_any(py)?,
                    (planet.get_orbital_radius() * 40000.0 < star.get_dyson_radius() as f32).into_bound_py_any(py)?,
                    satellite_count                                                         .into_bound_py_any(py)?,
                    planet.get_theme().temperature                                          .into_bound_py_any(py)?,
                    planet.get_theme().id                                                   .into_bound_py_any(py)?,
                    gas_h                                                                   .into_bound_py_any(py)?,
                    gas_d                                                                   .into_bound_py_any(py)?,
                    gas_i                                                                   .into_bound_py_any(py)?,
                    (planet.get_rotation_period() == planet.get_orbital_period())           .into_bound_py_any(py)?,
                ];
                let veins = planet.get_veins();
                let full_planet: Vec<&Bound<'_, PyAny>> = planet_line
                    .iter()
                    .chain(if planet.get_type() == &PlanetType::Gas {
                        repeat_n(&PyFloat::new(py, -1.0).into_bound_py_any(py)?, 42).into_iter()
                    }
                    else {
                        ORES
                            .iter()
                            .flat_map(|ore|{
                                for vein in veins {
                                    
                                }
                            })
                            .into_iter()
                        //repeat_n(&3.0.into_bound_py_any(py)?, 42)
                    })
                    .collect();
                planets.push(full_planet);
            }

        }

        Ok(PyTuple::empty(py))



        /*pythonize(py, &galaxy_lines).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to pythonize galaxy: {}", e))
        })?.into_py_any(py)*/
    }

}
