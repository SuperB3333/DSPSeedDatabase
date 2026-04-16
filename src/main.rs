mod data;
mod worldgen;

use postgres::{Client, CopyInWriter, NoTls};
use std::io::Write;

use crate::data::enums::{PlanetType, ORES};
use crate::data::game_desc::GameDesc;
use crate::worldgen::galaxy_gen::create_galaxy;
fn insert_seed(scopy: &mut CopyInWriter, pcopy: &mut CopyInWriter, seed: i32, star_count: usize, resource_multiplier: f32) -> Result<(), Box<dyn std::error::Error>> {

    let mut game_desc: GameDesc = GameDesc::default();
    game_desc.seed = seed;
    game_desc.star_count = star_count;
    game_desc.resource_multiplier = resource_multiplier;
    let galaxy = create_galaxy(&game_desc);

    for solar_system in galaxy.stars {
        let star = solar_system.star.clone();
        let star_id = star.index as i32 + seed * 100;

        write!(scopy, "{},", star_id)?;
        write!(scopy, "{},", seed)?;
        write!(scopy, "{},", star.position.magnitude())?; //todo change to magnitude_sq and change rule to account for that
        write!(scopy, "{},", star.index)?;
        write!(scopy, "{},", star.get_luminosity())?;
        write!(scopy, "{},", star.get_dyson_radius())?;
        write!(scopy, "{},", star.star_type.clone() as i32 + 1)?;
        write!(scopy, "{},", star.get_spectr().clone() as i32)?;

        for (index, ore) in ORES[1..15].iter().enumerate() {
            write!(scopy, "{}{}", solar_system.get_avg_vein(ore) as i32, if index == 13 {"\n"} else {","})?;
        }

        for planet in solar_system.get_planets() {
            let satellite_count = -1; //todo implement

            let mut gas_h = &0.0;
            let mut gas_d = &0.0;
            let mut gas_i = &0.0;
            if planet.get_type() == &PlanetType::Gas {
                for (gas, rate) in planet.get_gases() {
                    match gas {
                        1120 => gas_h = rate,
                        1121 => gas_d = rate,
                        1011 => gas_i = rate,
                        _ => panic!("Illegal state: gas was not 1120, 1121 or 1011! This should never happen.")
                    }
                }
            }
            write!(pcopy, "{},", star_id)?;
            write!(pcopy, "{},", planet.index)?;
            write!(pcopy, "{},", planet.get_theme().water_item_id)?;
            write!(pcopy, "{},", planet.get_type() == &PlanetType::Gas)?;
            write!(pcopy, "{},", planet.get_orbital_radius())?;
            write!(pcopy, "{},", planet.get_orbital_radius() * 40000.0 < star.get_dyson_radius() as f32)?;
            write!(pcopy, "{},", satellite_count)?;
            write!(pcopy, "{},", planet.get_theme().temperature)?;
            write!(pcopy, "{},", planet.get_theme().id)?;
            write!(pcopy, "{},{},{},", gas_h, gas_d, gas_i)?;
            write!(pcopy, "{},", planet.get_rotation_period() == planet.get_orbital_period())?;

            let veins = planet.get_veins();

            if planet.get_type() == &PlanetType::Gas {
                for _ in 0..41 {
                    write!(pcopy, "-1,")?;
                }
                write!(pcopy, "-1")?;
            } else {
                for (index, ore) in ORES[1..15].iter().enumerate() {
                    let mut found = false;
                    for vein in veins {
                        if vein.vein_type == *ore {
                            write!(pcopy, "{},", vein.min())?;
                            write!(pcopy, "{},", vein.max())?;
                            write!(pcopy, "{}{}", vein.estimate(), if index == 13 {"\n"} else {","})?; // if it is the last entry, skip the comma
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        write!(pcopy, "-1,-1,-1{}", if index == 13 {"\n"} else {","})?;
                    }
                }
            }
        }
    }
    Ok(())
}

const START_SEED: i32 = 0;
const END_SEED: i32 = 1000;
const STAR_COUNT: usize = 64;
const REC_MULTIPLIER: f32 = 1.0;


const COPY_PLANET: &str = "COPY planets(star_id, index, water_item, gas_giant, sun_distance, inside_ds, satellites, temperature, theme_id, gas_h, gas_d, gas_i, tidal_lock, min_iron, max_iron, estimate_iron, min_copper, max_copper, estimate_copper, min_silicium, max_silicium, estimate_silicium, min_titanium, max_titanium, estimate_titanium, min_stone, max_stone, estimate_stone, min_coal, max_coal, estimate_coal, min_oil, max_oil, estimate_oil, min_fireice, max_fireice, estimate_fireice, min_diamond, max_diamond, estimate_diamond, min_fractal, max_fractal, estimate_fractal, min_crysrub, max_crysrub, estimate_crysrub, min_grat, max_grat, estimate_grat, min_bamboo, max_bamboo, estimate_bamboo, min_mag, max_mag, estimate_mag) FROM STDIN WITH (FORMAT CSV)";
const COPY_STAR: &str = "COPY stars(id, seed, start_dist, star_index, luminosity, dyson_radius, type, spectr, ore_iron, ore_copper, ore_silicium, ore_titanium, ore_stone, ore_coal, ore_oil, ore_fireice, ore_diamond, ore_fractal, ore_crysrub, ore_grat, ore_bamboo, ore_mag) FROM STDIN WITH (FORMAT CSV)";
fn main() {
    let mut star_client = Client::connect("postgres://postgres:rootpassword@localhost:5432/dsp?sslmode=disable", NoTls).unwrap();
    let mut planet_client = Client::connect("postgres://postgres:rootpassword@localhost:5432/dsp?sslmode=disable", NoTls).unwrap();

    let mut scpy = star_client.copy_in(COPY_STAR).unwrap();
    let mut pcpy = planet_client.copy_in(COPY_PLANET).unwrap();

    for seed in START_SEED..END_SEED {
        insert_seed(&mut scpy, &mut pcpy, seed, STAR_COUNT, REC_MULTIPLIER).expect("insert_seed failed");
    }
    scpy.finish().unwrap();
    pcpy.finish().unwrap();
}
