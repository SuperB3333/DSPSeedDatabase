use crate::misc::{create_db_schema, split_chunks};
use crate::{
    log_info, log_warn, BENCHMARK, CHECKPOINT_FILE, END_SEED, PROGRESS_WORKERS, START_SEED,
    TEST_ONLY, WORKER_THREADS,
};
use anyhow::Result;
use std::fs::OpenOptions;
use std::io::Write;
use std::ops::Range;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::LazyLock;
static MAX_BUFFER: LazyLock<i32> = LazyLock::new(|| 2);

pub fn write_checkpoints() -> Result<()> {
    let mut cfile = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&*CHECKPOINT_FILE)?;
    PROGRESS_WORKERS
        .iter()
        .map(|i| i.load(Relaxed) - *MAX_BUFFER)
        .for_each(|x| {
            writeln!(cfile, "{}", x).unwrap();
        });
    Ok(())
}
/// Loads potential checkpoints or defaults to full seeds. Also creates db schema and looks for potential corrupted data
pub fn load_workloads() -> Result<Vec<Range<i32>>> {
    let normal_workloads = split_chunks(*START_SEED..*END_SEED, *WORKER_THREADS);
    let end_seeds = normal_workloads.iter().map(|i| i.end).collect();

    let (workloads, had_checkpoints) = match read_checkpoints(end_seeds) {
        Some(workloads) => {
            log_info!("Read checkpoints successfully");
            (workloads, true)
        }
        None => {
            log_info!("No checkpoints found; distributing all seeds among generators");
            (normal_workloads, false)
        }
    };

    if !*BENCHMARK && !*TEST_ONLY && !(create_db_schema() == had_checkpoints) {
        log_warn!(
            "Found checkpoints and no db schema or the other way around. Data might be corrupt!"
        )
    }

    Ok(workloads)
}
fn read_checkpoints(ends: Vec<i32>) -> Option<Vec<Range<i32>>> {
    let _cp_path = &**CHECKPOINT_FILE;
    if !std::fs::exists(_cp_path).unwrap_or(false) { return None }
    let contents = match std::fs::read_to_string(_cp_path) {
        Ok(contents) => contents,
        Err(_) => return None
    };

    Some(
        contents
            .split("\n")
            .map(|s| s.parse::<i32>().unwrap())
            .zip(ends)
            .map(|i| i.0..i.1)
            .collect()
    )
}
