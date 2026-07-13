use crate::misc::{split_chunks, COPY_PLANET, COPY_STAR};
use crate::{COMMITTED_SEEDS, COMMIT_COUNT, DB_STR, END_SEED, START_SEED, WORKER_THREADS};
use anyhow::Result;
use crossbeam_channel::{Receiver, RecvTimeoutError};
use postgres::{Client, NoTls};
use std::io::Write;
use std::ops::Range;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::LazyLock;
use std::time::Duration;

static ENABLED: LazyLock<bool> = LazyLock::new(|| {
    std::env::var("TEST_ONLY")
        .map(|value| value == "1")
        .unwrap_or(false)
});

pub fn enabled() -> bool {
    *ENABLED
}

pub fn workloads() -> Vec<Range<i32>> {
    split_chunks(*START_SEED..*END_SEED, *WORKER_THREADS)
}

pub fn rollback_writer(rec: Receiver<(String, String)>) -> Result<()> {
    let mut client = Client::connect(&*DB_STR.as_str(), NoTls)?;

    'outer: loop {
        let mut batch: Vec<(String, String)> = Vec::with_capacity(*COMMIT_COUNT);
        'inner: for index in 0..*COMMIT_COUNT {
            match rec.recv_timeout(Duration::from_secs(1)) {
                Ok(message) => batch.push(message),
                Err(RecvTimeoutError::Timeout) => {
                    panic!("rollback_writer: channel stall detected (>1s lull)")
                }
                Err(RecvTimeoutError::Disconnected) => {
                    if index == 0 {
                        break 'outer;
                    }
                    break 'inner;
                }
            }
        }

        let mut transaction = client.transaction()?;
        {
            let mut copy = transaction.copy_in(COPY_STAR)?;
            for (star, _) in &batch {
                copy.write_all(star.as_bytes())?;
            }
            copy.finish()?;
        }
        {
            let mut copy = transaction.copy_in(COPY_PLANET)?;
            for (_, planet) in &batch {
                copy.write_all(planet.as_bytes())?;
            }
            copy.finish()?;
        }
        transaction.rollback()?;
        COMMITTED_SEEDS.fetch_add(batch.len() as i32, SeqCst);
    }
    Ok(())
}
