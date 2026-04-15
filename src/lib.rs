mod data;
mod worldgen;


use pyo3::prelude::*;
#[pymodule]
mod dsp_generator {
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
    fn generate(py: Python, seed: i32, star_count: usize, resource_multiplier: f32) -> PyResult<Bound<'_, PyAny>> {
        let mut game_desc: GameDesc = GameDesc::default();
        game_desc.seed = seed; game_desc.star_count = star_count; game_desc.resource_multiplier = resource_multiplier;
        let galaxy = create_galaxy(&game_desc);
        pythonize(py, &galaxy.stars).map_err(|e| {
            PyErr::new::<PyRuntimeError, _>(format!("Failed to pythonize galaxy: {}", e))
        })
    }
    #[pyfunction]
    fn generate_formatted(py: Python<'_>, seed: i32, star_count: usize, resource_multiplier: f32) -> PyResult<Bound<'_, PyTuple>> {
        let mut game_desc: GameDesc = GameDesc::default();
        game_desc.seed = seed; game_desc.star_count = star_count; game_desc.resource_multiplier = resource_multiplier;
        let galaxy = create_galaxy(&game_desc);
        let mut stars = vec![];
        let mut planets = vec![];

        for solar_system in galaxy.stars {
            let star = solar_system.star.clone();
            let star_id = star.index as i32 + seed * 100;

            let mut star_line = vec![
                star_id                               .into_bound_py_any(py)?,
                seed                                  .into_bound_py_any(py)?,
                star.position.magnitude()             .into_bound_py_any(py)?,
                star.index                            .into_bound_py_any(py)?,
                star.get_luminosity()                 .into_bound_py_any(py)?,
                star.get_dyson_radius()               .into_bound_py_any(py)?,
                (star.star_type.clone() as i32 + 1)   .into_bound_py_any(py)?,
                (star.get_spectr().clone() as i32)    .into_bound_py_any(py)?,
            ];
            for ore in &ORES[1..15] {
                star_line.push((solar_system.get_avg_vein(ore) as i32).into_bound_py_any(py)?);
            }
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
                let mut full_planet = planet_line;

                if planet.get_type() == &PlanetType::Gas {
                    for _ in 0..42 {
                        full_planet.push((-1).into_bound_py_any(py)?);
                    }
                }
                else {
                    for ore in &ORES[1..15] {
                        let mut found = false;
                        for vein in veins {
                            if vein.vein_type == *ore {
                                full_planet.push(vein.min().into_bound_py_any(py)?);
                                full_planet.push(vein.max().into_bound_py_any(py)?);
                                full_planet.push(vein.estimate().into_bound_py_any(py)?);
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            for _ in 0..3 {
                                full_planet.push((-1).into_bound_py_any(py)?);
                            }
                        }
                    }
                }
                planets.push(full_planet);
            }

        }

        PyTuple::new(py, [stars, planets])
    }

}
