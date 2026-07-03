use std::io::Write;
use std::ops::Range;
use crate::macros::env_str;

pub const COPY_PLANET: &str = "COPY planets(star_id, index, orbiting, water_item, gas_giant, sun_distance, inside_ds, satellites, temperature, theme_id, gas_h, gas_d, gas_i, tidal_lock, min_iron, max_iron, estimate_iron, min_copper, max_copper, estimate_copper, min_silicium, max_silicium, estimate_silicium, min_titanium, max_titanium, estimate_titanium, min_stone, max_stone, estimate_stone, min_coal, max_coal, estimate_coal, min_oil, max_oil, estimate_oil, min_fireice, max_fireice, estimate_fireice, min_diamond, max_diamond, estimate_diamond, min_fractal, max_fractal, estimate_fractal, min_crysrub, max_crysrub, estimate_crysrub, min_grat, max_grat, estimate_grat, min_bamboo, max_bamboo, estimate_bamboo, min_mag, max_mag, estimate_mag) FROM STDIN WITH (FORMAT CSV)";
pub const COPY_STAR: &str = "COPY stars(id, seed, start_dist, star_index, luminosity, dyson_radius, type, spectr, ore_iron, ore_copper, ore_silicium, ore_titanium, ore_stone, ore_coal, ore_oil, ore_fireice, ore_diamond, ore_fractal, ore_crysrub, ore_grat, ore_bamboo, ore_mag) FROM STDIN WITH (FORMAT CSV)";


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

/// Atomically persist per-worker checkpoint values.
///
/// The values are written to a sibling temp file (`<path>.tmp`) which is then
/// flushed, fsync'd, and renamed over the real checkpoint file. On any sane
/// filesystem `rename` is atomic, so a reader (including a resuming run) always
/// observes either the previous complete checkpoint or the new complete one,
/// never a truncated/partial file. The written values are identical to the
/// previous in-place implementation, so resume behaviour is unchanged.
pub fn write_checkpoint_atomic(path: &str, values: &[i32]) -> std::io::Result<()> {
    let tmp_path = format!("{}.tmp", path);
    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)?;
        let mut buf = String::with_capacity(values.len() * 8);
        for v in values {
            buf.push_str(itoa_line(*v).as_str());
        }
        f.write_all(buf.as_bytes())?;
        f.flush()?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp_path, path)
}

#[inline]
fn itoa_line(v: i32) -> String {
    let mut s = v.to_string();
    s.push('\n');
    s
}

pub fn get_db_str() -> String {
    let user = env_str!("PG_USER", "postgres");
    let pass = env_str!("PG_PASS", "rootpassword");
    let netloc = env_str!("PG_NETLOC", "localhost");
    let port = env_str!("PG_PORT", "5432");
    let db_name = env_str!("PG_DBNAME", "dsp");
    format!("postgres://{user}:{pass}@{netloc}:{port}/{db_name}?sslmode=disable")
}