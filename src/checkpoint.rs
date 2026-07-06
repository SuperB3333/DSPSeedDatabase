use std::fs::OpenOptions;
use anyhow::Result;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::LazyLock;
use std::io::Write;
use std::ops::Range;
use crate::{CHECKPOINT_FILE, END_SEED, PROGRESS_WORKERS, START_SEED, WORKER_THREADS};
use crate::misc::split_chunks;

static MAX_BUFFER: LazyLock<i32> = LazyLock::new(|| {
    2
});


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
pub fn load_workloads() -> Result<Vec<Range<i32>>> {
    return Ok(split_chunks(*START_SEED..*END_SEED, *WORKER_THREADS));
    todo!("load workloads from checkpoints file")
}