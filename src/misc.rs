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

/// One worker's recorded chunk plus committed watermark.
///
/// `start`/`end` are the exact `Range<i32>` (chunk) that worker was assigned.
/// `watermark` is the highest `seed + 1` in that chunk that is *guaranteed*
/// committed to the database (conservative — see the monitor loop in main.rs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkerCheckpoint {
    pub start: i32,
    pub end: i32,
    pub watermark: i32,
}

/// Parsed v2 checkpoint file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub start_seed: i32,
    pub end_seed: i32,
    pub worker_count: i32,
    pub workers: Vec<WorkerCheckpoint>,
}

/// Atomically persist a v2 checkpoint.
///
/// The payload is written to a sibling temp file (`<path>.tmp`) which is then
/// flushed, fsync'd, and renamed over the real checkpoint file. On any sane
/// filesystem `rename` is atomic, so a reader (including a resuming run) always
/// observes either the previous complete checkpoint or the new complete one,
/// never a truncated/partial file. If a kill occurs mid-write, only the `.tmp`
/// file may be left behind; the reader ignores it.
///
/// Format:
/// ```text
/// v2 <start_seed> <end_seed> <worker_count>
/// <chunk_start> <chunk_end> <watermark>     # one line per worker
/// ```
pub fn write_checkpoint_atomic(path: &str, cp: &Checkpoint) -> std::io::Result<()> {
    let tmp_path = format!("{}.tmp", path);
    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)?;
        let mut buf = String::with_capacity(64 + cp.workers.len() * 24);
        buf.push_str(&format!(
            "v2 {} {} {}\n",
            cp.start_seed, cp.end_seed, cp.worker_count
        ));
        for w in &cp.workers {
            buf.push_str(&format!("{} {} {}\n", w.start, w.end, w.watermark));
        }
        f.write_all(buf.as_bytes())?;
        f.flush()?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp_path, path)
}

/// Read + parse a v2 checkpoint file.
///
/// Returns:
/// - `Ok(Some(cp))`  — a well-formed v2 checkpoint was read.
/// - `Ok(None)`      — the file does not exist (fresh start).
/// - `Err(msg)`      — the file exists but is not a parseable v2 checkpoint;
///                     the caller treats this as a fresh start (with a debug log).
pub fn read_checkpoint(path: &str) -> Result<Option<Checkpoint>, String> {
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("cannot read checkpoint file: {}", e)),
    };

    let mut lines = data.lines();
    let header = lines.next().ok_or_else(|| "empty checkpoint file".to_string())?;
    let mut h = header.split_whitespace();
    match h.next() {
        Some("v2") => {}
        other => return Err(format!("unexpected checkpoint version: {:?}", other)),
    }
    let start_seed = parse_field(h.next(), "start_seed")?;
    let end_seed = parse_field(h.next(), "end_seed")?;
    let worker_count = parse_field(h.next(), "worker_count")?;

    let mut workers = Vec::with_capacity(worker_count.max(0) as usize);
    for _ in 0..worker_count {
        let line = lines
            .next()
            .ok_or_else(|| "checkpoint has fewer worker lines than worker_count".to_string())?;
        let mut p = line.split_whitespace();
        let start = parse_field(p.next(), "chunk_start")?;
        let end = parse_field(p.next(), "chunk_end")?;
        let watermark = parse_field(p.next(), "watermark")?;
        workers.push(WorkerCheckpoint { start, end, watermark });
    }

    Ok(Some(Checkpoint {
        start_seed,
        end_seed,
        worker_count,
        workers,
    }))
}

#[inline]
fn parse_field(v: Option<&str>, name: &str) -> Result<i32, String> {
    v.ok_or_else(|| format!("missing checkpoint field: {}", name))?
        .parse::<i32>()
        .map_err(|e| format!("bad checkpoint field {}: {}", name, e))
}

pub fn get_db_str() -> String {
    let user = env_str!("PG_USER", "postgres");
    let pass = env_str!("PG_PASS", "rootpassword");
    let netloc = env_str!("PG_NETLOC", "localhost");
    let port = env_str!("PG_PORT", "5432");
    let db_name = env_str!("PG_DBNAME", "dsp");
    format!("postgres://{user}:{pass}@{netloc}:{port}/{db_name}?sslmode=disable")
}