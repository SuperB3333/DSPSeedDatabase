#[path = "../algorithm/mod.rs"]
mod algorithm;

use algorithm::data::enums::{ThemeDistribute, VeinType, ORES};
use algorithm::data::game_desc::GameDesc;
use algorithm::data::planet::Planet;
use algorithm::data::random::DspRandom;
use algorithm::data::vector_f3::VectorF3;
use algorithm::data::vein::EstimatedVein;
use algorithm::generate_stars;
use anyhow::{bail, Context, Result};
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

const STAR_COUNT: usize = 64;
const REC_MULTIPLIER: f32 = 1.0;
const HELD_OUT_START: i32 = 1_000_000;
const HELD_OUT_END: i32 = 1_001_000;
const ORE_NAMES: [&str; 14] = [
    "iron", "copper", "silicium", "titanium", "stone", "coal", "oil", "fireice", "diamond",
    "fractal", "crysrub", "grat", "bamboo", "mag",
];

fn parse_seed(value: Option<String>, name: &str) -> Result<i32> {
    value
        .with_context(|| format!("missing {name}"))?
        .parse()
        .with_context(|| format!("invalid {name}"))
}

fn write_header(writer: &mut impl Write) -> Result<()> {
    write!(
        writer,
        "seed,star_index,planet_index,planet_seed,theme_id,algorithm_id,star_type,spectr,gas"
    )?;
    for ore in ORE_NAMES {
        write!(writer, ",estimate_{ore}")?;
    }
    for ore in ORE_NAMES {
        write!(writer, ",spacing_{ore}")?;
    }
    for ore in ORE_NAMES {
        write!(writer, ",actual_{ore}")?;
    }
    writeln!(writer)?;
    Ok(())
}

fn base_amounts(veins: &[EstimatedVein]) -> ([i64; 16], [i32; 16]) {
    let mut amounts = [0_i64; 16];
    let mut spots = [0_i32; 16];
    for vein in veins {
        let index = vein.vein_type as usize;
        amounts[index] = vein.estimate();
        spots[index] = vein.min_group + 1;
    }
    (amounts, spots)
}

fn estimate_spacing(planet: &Planet<'_>, veins: &[EstimatedVein]) -> [i64; 16] {
    let (mut amounts, spots) = base_amounts(veins);
    if planet.get_theme().distribute == ThemeDistribute::Birth {
        return amounts;
    }

    let mut source = DspRandom::new(planet.seed);
    for _ in 0..5 {
        source.advance();
    }
    let mut random = DspRandom::new(source.next_seed());
    let mut birth_point = VectorF3::new(
        (random.next_f64() * 2.0 - 1.0) as f32,
        (random.next_f64() - 0.5) as f32,
        (random.next_f64() * 2.0 - 1.0) as f32,
    );
    birth_point.normalize();
    birth_point *= (random.next_f64() * 0.4 + 0.2) as f32;

    let mut centers = [VectorF3::zero(); 512];
    let mut center_count = 0;
    let min_spacing = 2.1 / planet.radius;
    let min_spacing_sq = (min_spacing as f64) * (min_spacing as f64);
    for ore in &ORES[1..15] {
        let index = *ore as usize;
        let nominal = spots[index];
        if nominal == 0 {
            continue;
        }
        let requested = if nominal > 1 {
            nominal + random.next_i32(3) - 1
        } else {
            nominal
        };
        let threshold = min_spacing_sq * if ore == &VeinType::Oil { 100.0 } else { 196.0 };
        let mut accepted = 0;
        for _ in 0..requested {
            for _ in 0..200 {
                let mut direction = VectorF3::new(
                    (random.next_f64() * 2.0 - 1.0) as f32,
                    (random.next_f64() * 2.0 - 1.0) as f32,
                    (random.next_f64() * 2.0 - 1.0) as f32,
                );
                if ore != &VeinType::Oil {
                    direction += birth_point;
                }
                direction.normalize();
                if centers[..center_count]
                    .iter()
                    .all(|center| center.distance_sq_from(&direction) as f64 >= threshold)
                {
                    if center_count == centers.len() {
                        break;
                    }
                    centers[center_count] = direction;
                    center_count += 1;
                    accepted += 1;
                    break;
                }
            }
        }
        amounts[index] = (amounts[index] * accepted as i64 + nominal as i64 / 2) / nominal as i64;
    }
    amounts
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let start_seed = parse_seed(args.next(), "start seed")?;
    let end_seed = parse_seed(args.next(), "end seed")?;
    let output = args.next().context("missing output path")?;
    let allow_held_out = match args.next().as_deref() {
        None => false,
        Some("--allow-held-out") if args.next().is_none() => true,
        Some(_) => bail!("expected only the optional --allow-held-out flag"),
    };
    if start_seed >= end_seed {
        bail!("start seed must be less than end seed");
    }
    if start_seed < HELD_OUT_END && end_seed > HELD_OUT_START && !allow_held_out {
        bail!("held-out seed range requires --allow-held-out");
    }

    let file = File::create(Path::new(&output))
        .with_context(|| format!("cannot create output file: {output}"))?;
    let mut writer = BufWriter::new(file);
    write_header(&mut writer)?;
    let game_desc = GameDesc {
        star_count: STAR_COUNT,
        resource_multiplier: REC_MULTIPLIER,
    };

    for seed in start_seed..end_seed {
        let habitable_count = std::cell::Cell::new(0);
        for solar_system in generate_stars(seed, &game_desc, &habitable_count) {
            let star = &solar_system.star;
            for planet in solar_system.get_planets() {
                let estimated_veins = planet.get_estimated_veins();
                let (estimates, _) = base_amounts(estimated_veins);
                let spacing = estimate_spacing(planet, estimated_veins);
                let mut actual = [0_i32; 16];
                for vein in planet.get_actual_veins() {
                    actual[vein.vein_type as usize] = vein.amount;
                }
                write!(
                    writer,
                    "{},{},{},{},{},{},{},{},{}",
                    seed,
                    star.index,
                    planet.index,
                    planet.seed,
                    planet.get_theme().id,
                    planet.get_algo() as i32,
                    star.star_type as i32,
                    star.get_spectr() as i32,
                    planet.is_gas_giant() as u8,
                )?;
                for ore in &ORES[1..15] {
                    write!(writer, ",{}", estimates[*ore as usize])?;
                }
                for ore in &ORES[1..15] {
                    write!(writer, ",{}", spacing[*ore as usize])?;
                }
                for ore in &ORES[1..15] {
                    write!(writer, ",{}", actual[*ore as usize])?;
                }
                writeln!(writer)?;
            }
        }
    }
    writer.flush()?;
    Ok(())
}

fn main() -> Result<()> {
    run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_has_metadata_and_three_fields_per_ore() {
        let mut output = Vec::new();
        write_header(&mut output).unwrap();
        let header = String::from_utf8(output).unwrap();
        let fields: Vec<_> = header.trim_end().split(',').collect();

        assert_eq!(fields.len(), 9 + ORE_NAMES.len() * 3);
        assert_eq!(fields[0], "seed");
        assert_eq!(fields[9], "estimate_iron");
        assert_eq!(fields[fields.len() - 1], "actual_mag");
    }

    #[test]
    fn spacing_estimate_has_a_deterministic_fixture() {
        let game_desc = GameDesc {
            star_count: STAR_COUNT,
            resource_multiplier: REC_MULTIPLIER,
        };
        let habitable_count = std::cell::Cell::new(0);
        let systems = generate_stars(0, &game_desc, &habitable_count);
        let planet = &systems[0].get_planets()[2];

        assert_eq!(
            estimate_spacing(planet, planet.get_estimated_veins()),
            [
                0, 5_148_000, 148_500, 4_620_000, 15_972_000, 2_310_000, 89_100, 0, 2_904_000, 0,
                0, 0, 0, 0, 0, 0,
            ]
        );
    }
}
