#[path = "../algorithm/mod.rs"]
mod algorithm;
#[path = "../generate_csv.rs"]
mod generate_csv;

use anyhow::{bail, Context, Result};
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

const STAR_COUNT: usize = 64;
const REC_MULTIPLIER: f32 = 1.0;
const OUTPUT_MAGIC: &[u8] = b"DSPSEED1";

fn parse_seed(value: Option<String>, name: &str) -> Result<i32> {
    value
        .with_context(|| format!("missing {name}"))?
        .parse()
        .with_context(|| format!("invalid {name}"))
}

fn write_record(writer: &mut impl Write, seed: i32, data: &[u8]) -> Result<()> {
    writer.write_all(&seed.to_be_bytes())?;
    writer.write_all(&(data.len() as u64).to_be_bytes())?;
    writer.write_all(data)?;
    Ok(())
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let start_seed = parse_seed(args.next(), "start seed")?;
    let end_seed = parse_seed(args.next(), "end seed")?;
    let output = args.next().context("missing output path")?;
    if args.next().is_some() {
        bail!("too many arguments");
    }
    if start_seed >= end_seed {
        bail!("start seed must be less than end seed");
    }

    let file = File::create(Path::new(&output))
        .with_context(|| format!("cannot create output file: {output}"))?;
    let mut writer = BufWriter::new(file);
    writer.write_all(OUTPUT_MAGIC)?;
    for seed in start_seed..end_seed {
        let (stars, planets) = generate_csv::gen_formatted(seed)?;
        write_record(&mut writer, seed, &stars)?;
        write_record(&mut writer, seed, &planets)?;
    }
    writer.flush()?;
    Ok(())
}

fn main() -> Result<()> {
    run()
}
