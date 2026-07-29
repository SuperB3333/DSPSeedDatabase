use crate::algorithm::data;
use crate::algorithm::data::enums::{PlanetType, ORES};
use crate::algorithm::data::game_desc::GameDesc;
use crate::algorithm::generate_stars;

pub const COPY_SIGNATURE: &[u8] = b"PGCOPY\n\xFF\r\n\0";
pub const COPY_HEADER_FIELDS: &[u8] = &[0; 8];
pub const COPY_FOOTER: &[u8] = b"\xFF\xFF";

#[inline]
fn write_row_header(buf: &mut Vec<u8>, columns: i16) {
    buf.extend_from_slice(&columns.to_be_bytes());
}

#[inline]
fn write_i32(buf: &mut Vec<u8>, v: i32) {
    buf.extend_from_slice(&4i32.to_be_bytes());
    buf.extend_from_slice(&v.to_be_bytes());
}
#[inline]
fn write_i16(buf: &mut Vec<u8>, v: i16) {
    buf.extend_from_slice(&2i32.to_be_bytes());
    buf.extend_from_slice(&v.to_be_bytes());
}
#[inline]
fn write_f32(buf: &mut Vec<u8>, v: f32) {
    buf.extend_from_slice(&4i32.to_be_bytes());
    buf.extend_from_slice(&v.to_be_bytes());
}
#[inline]
fn write_bool(buf: &mut Vec<u8>, v: bool) {
    buf.extend_from_slice(&1i32.to_be_bytes());
    buf.push(v as u8);
}
#[inline]
fn write_null_col(buf: &mut Vec<u8>) {
    buf.extend_from_slice(&(-1i32).to_be_bytes());
}

pub fn gen_formatted(seed: i32) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let game_desc: GameDesc = GameDesc {
        star_count: crate::STAR_COUNT,
        resource_multiplier: crate::REC_MULTIPLIER,
    };
    let hab_count = std::cell::Cell::new(0i32);
    let galaxy = generate_stars(seed, &game_desc, &hab_count);

    let mut stars: Vec<u8> = Vec::with_capacity(crate::STAR_COUNT * 176);
    let mut planets: Vec<u8> = Vec::with_capacity(crate::STAR_COUNT * 256 * 5);

    for solar_system in galaxy {
        let star = solar_system.star.clone();
        let star_id = star.index as i32 + seed * 100;

        // Stars: 22 columns
        // Columns: id, seed, start_dist, star_index, luminosity, dyson_radius, type, spectr, ore_iron..ore_mag
        write_row_header(&mut stars, 22);
        write_i32(&mut stars, star_id);
        write_i32(&mut stars, seed);
        write_f32(&mut stars, star.position.magnitude() as f32);
        write_i16(&mut stars, star.index as i16);
        write_f32(&mut stars, star.get_luminosity());
        write_i32(&mut stars, star.get_dyson_radius());
        write_i16(&mut stars, star.star_type as i16 + 1);
        write_i16(&mut stars, star.get_spectr() as i16);

        for ore in ORES[1..15].iter() {
            write_i32(&mut stars, solar_system.get_avg_vein(ore) as i32);
        }

        let mut satellite_counts = std::collections::HashMap::new();
        for planet in solar_system.get_planets() {
            if planet.has_orbit_around() {
                *satellite_counts.entry(planet.orbit_around.borrow().as_ref().unwrap().index).or_insert(0) += 1;
            }
        }

        for planet in solar_system.get_planets() {
            let satellite_count = if planet.is_gas_giant() {
                satellite_counts.get(&planet.index).copied().unwrap_or(0)
            } else {
                0
            };

            let orbiting = if planet.has_orbit_around() {
                planet.orbit_around.borrow().as_ref().unwrap().index as i16
            } else {
                -1i16
            };

            let mut gas_h = 0.0f32;
            let mut gas_d = 0.0f32;
            let mut gas_i = 0.0f32;
            if planet.get_type() == &PlanetType::Gas {
                for (gas, rate) in planet.get_gases() {
                    match gas {
                        1120 => gas_h = *rate,
                        1121 => gas_d = *rate,
                        1011 => gas_i = *rate,
                        _ => panic!("Illegal state: gas was not 1120, 1121 or 1011! This should never happen.")
                    }
                }
            }

            // Planets: 26 columns
            // Columns: star_id, index, orbiting, gas_giant, sun_distance, inside_ds,
            //          satellites, theme_id, gas_h, gas_d, gas_i, tidal_lock,
            //          ore_iron..ore_mag
            let sun_distance = planet.get_sun_distance();
            write_row_header(&mut planets, 26);
            write_i32(&mut planets, star_id);
            write_i16(&mut planets, planet.index as i16);
            write_i16(&mut planets, orbiting);
            write_bool(&mut planets, planet.get_type() == &PlanetType::Gas);
            write_f32(&mut planets, sun_distance);
            write_bool(
                &mut planets,
                sun_distance * 40000.0 < star.get_dyson_radius() as f32,
            );
            write_i16(&mut planets, satellite_count);
            write_i16(&mut planets, planet.get_theme().id as i16);
            write_f32(&mut planets, gas_h);
            write_f32(&mut planets, gas_d);
            write_f32(&mut planets, gas_i);
            write_bool(&mut planets, planet.is_tidal_locked());

            let veins = planet.get_actual_veins();
            let mut vein_map: [Option<&data::vein::ActualVein>; 16] = [None; 16];
            for v in veins.iter() {
                vein_map[v.vein_type as usize] = Some(v);
            }

            if planet.get_type() == &PlanetType::Gas {
                for _ in 0..14 {
                    write_null_col(&mut planets);
                }
            } else {
                for ore in ORES[1..15].iter() {
                    if let Some(vein) = vein_map[*ore as usize] {
                        write_i32(&mut planets, vein.amount);
                    } else {
                        write_null_col(&mut planets);
                    }
                }
            }
        }
    }

    Ok((stars, planets))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_fields_include_lengths() {
        let mut output = Vec::new();
        write_i16(&mut output, 7);
        write_i32(&mut output, 11);
        write_f32(&mut output, 1.5);
        write_bool(&mut output, true);
        write_null_col(&mut output);

        assert_eq!(
            output,
            [
                0, 0, 0, 2, 0, 7, 0, 0, 0, 4, 0, 0, 0, 11, 0, 0, 0, 4, 63, 192, 0, 0, 0,
                0, 0, 1, 1, 255, 255, 255, 255,
            ]
        );
    }

    #[test]
    fn binary_copy_framing_has_protocol_lengths() {
        assert_eq!(COPY_SIGNATURE.len(), 11);
        assert_eq!(COPY_HEADER_FIELDS.len(), 8);
        assert_eq!(COPY_FOOTER, [255, 255]);
    }
}
