use std::ops::Range;

pub const COPY_PLANET: &str = "COPY planets(star_id, index, water_item, gas_giant, sun_distance, inside_ds, satellites, temperature, theme_id, gas_h, gas_d, gas_i, tidal_lock, min_iron, max_iron, estimate_iron, min_copper, max_copper, estimate_copper, min_silicium, max_silicium, estimate_silicium, min_titanium, max_titanium, estimate_titanium, min_stone, max_stone, estimate_stone, min_coal, max_coal, estimate_coal, min_oil, max_oil, estimate_oil, min_fireice, max_fireice, estimate_fireice, min_diamond, max_diamond, estimate_diamond, min_fractal, max_fractal, estimate_fractal, min_crysrub, max_crysrub, estimate_crysrub, min_grat, max_grat, estimate_grat, min_bamboo, max_bamboo, estimate_bamboo, min_mag, max_mag, estimate_mag) FROM STDIN WITH (FORMAT CSV)";
pub const COPY_STAR: &str = "COPY stars(id, seed, start_dist, star_index, luminosity, dyson_radius, type, spectr, ore_iron, ore_copper, ore_silicium, ore_titanium, ore_stone, ore_coal, ore_oil, ore_fireice, ore_diamond, ore_fractal, ore_crysrub, ore_grat, ore_bamboo, ore_mag) FROM STDIN WITH (FORMAT CSV)";

const INIT_SCRIPT: &str = include_str!("init.sql");

pub fn split_chunks(r: Range<i32>, chunks: i32) -> Vec<Range<i32>> {
    let total = r.end - r.start;
    let base = total / chunks;
    let mut extra = total % chunks;
    let mut cur = r.start;
    let mut out = Vec::with_capacity(chunks as usize);
    for _ in 0..chunks {
        let add = if extra > 0 { extra -= 1; base + 1 } else { base };
        out.push(cur..(cur + add));
        cur += add;
    }
    out
}
#[macro_export]
macro_rules! env_int {
    ($var:expr) => {
        match std::env::var($var) {
            Ok(val) => val.parse(),
            Err(err) => Err(err)
        }
    };
    ($var:expr, $default:expr) => {
        std::env::var($var).map(|e| e.parse::<i32>().unwrap_or($default)).unwrap_or($default)
    };
}
#[macro_export]
macro_rules! env_str {
    ($var:expr) => {
        std::env::var($var)
    };
    ($var:expr, $default:expr) => {
        std::env::var($var).unwrap_or($default.to_string())
    }
}

/// Creates the tables in the database, as well as the user for the api. Returns whether the database was set up before
pub fn create_db_schema() -> bool {
    postgres::Client::connect(
        (*crate::DB_STR).as_str(),
        postgres::NoTls
    )
        .unwrap()
        .batch_execute(INIT_SCRIPT)
        .is_err()
}