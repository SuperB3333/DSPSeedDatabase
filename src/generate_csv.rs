use crate::algorithm::data;
use crate::algorithm::data::enums::{PlanetType, ORES};
use crate::algorithm::data::game_desc::GameDesc;
use crate::algorithm::worldgen::galaxy_gen::create_galaxy;

pub fn gen_formatted(seed: i32, star_count: usize, resource_multiplier: f32) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut game_desc: GameDesc = GameDesc::default();
    game_desc.seed = seed;
    game_desc.star_count = star_count;
    game_desc.resource_multiplier = resource_multiplier;
    let galaxy = create_galaxy(&game_desc);

    let mut stars: String = String::with_capacity(star_count * 128);
    let mut planets: String = String::with_capacity(star_count * 256 * 5);

    for solar_system in galaxy.stars {
        let star = solar_system.star.clone();
        let star_id = star.index as i32 + seed * 100;

        stars.push_str(format!("{},{},{},{},{},{},{},{},",
                               star_id,
                               seed,
                               star.position.magnitude(),
                               star.index,
                               star.get_luminosity(),
                               star.get_dyson_radius(),
                               star.star_type as i32 + 1,
                               *star.get_spectr() as i32
        ).as_str());

        for (index, ore) in ORES[1..15].iter().enumerate() {
            stars.push_str(format!("{}{}", solar_system.get_avg_vein(ore) as i32, if index == 13 {"\n"} else {","}).as_str());
        }

        for planet in solar_system.get_planets() {
            let satellite_count = if !planet.is_gas_giant() { 0 } else {
                -1 //todo implement
            };


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

            planets.push_str(format!("{},{},{},{},{},{},{},{},{},{},{},{},{},",
                                     star_id,
                                     planet.index,
                                     planet.get_theme().water_item_id,
                                     planet.get_type() == &PlanetType::Gas,
                                     planet.get_orbital_radius(),
                                     planet.get_orbital_radius() * 40000.0 < star.get_dyson_radius() as f32,
                                     satellite_count,
                                     planet.get_theme().temperature,
                                     planet.get_theme().id,
                                     gas_h, gas_d, gas_i,
                                     planet.get_rotation_period() == planet.get_orbital_period()
            ).as_str());

            let veins = planet.get_veins();
            // OPTIMIZATION: Use fixed-size array indexed by VeinType discriminant instead of HashMap.
            // VeinType is #[repr(i32)] with variants None=0..Max=15, so a 16-element array gives
            // O(1) lookup with zero heap allocation, vs HashMap which allocates per-planet.
            // Impact: Eliminates ~256K HashMap allocations per 1000 seeds in the hot loop.
            let mut vein_map: [Option<&data::vein::Vein>; 16] = [None; 16];
            for v in veins.iter() {
                vein_map[v.vein_type as usize] = Some(v);
            }

            if planet.get_type() == &PlanetType::Gas {
                for _ in 0..41 {
                    planets.push_str("-1,");
                }
                planets.push_str("-1\n");
            } else {
                for (index, ore) in ORES[1..15].iter().enumerate() {
                    if let Some(vein) = vein_map[*ore as usize] {
                        planets.push_str(format!("{},{},{}{}",
                                                 vein.min(),
                                                 vein.max(),
                                                 vein.estimate(),
                                                 if index == 13 { "\n" } else { "," }
                        ).as_str());
                    } else {
                        planets.push_str(format!("-1,-1,-1{}", if index == 13 { "\n" } else { "," }).as_str());
                    }
                }
            }
        }
    }
    Ok((stars, planets))
}