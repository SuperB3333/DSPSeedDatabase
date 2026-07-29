use std::ops::Range;
use std::sync::atomic::Ordering::Relaxed;
use std::time::Duration;
use std::io::Write;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use postgres::{Client, NoTls};
use anyhow::Result;
use crate::generate_csv::gen_formatted;
use crate::{log_info, COMMITTED_SEEDS, COMMIT_COUNT, DB_STR, PROGRESS_WORKERS};
use crate::misc::{COPY_PLANET, COPY_STAR};

pub fn worker_thread(seeds: Range<i32>, send: Sender<(String, String)>, id: usize) -> Result<()> {
    for seed in seeds {
        let entry = gen_formatted(seed).expect("gen_formatted failed");
        send.send(entry)?;
        PROGRESS_WORKERS[id].fetch_add(1, Relaxed);

    }
    log_info!("Worker thread {} finished sucessfully", id);
    Ok(())
}
pub fn commit_thread(rec: Receiver<(String, String)>) -> Result<()> {
    let mut client = Client::connect(&*DB_STR.as_str(), NoTls)?;

    'outer: loop {
        let mut batch: Vec<(String, String)> = Vec::with_capacity(*COMMIT_COUNT);
        'inner: for i in 0..*COMMIT_COUNT {
            match rec.recv_timeout(Duration::new(1, 0)) {
                Ok(msg) => batch.push(msg),
                Err(RecvTimeoutError::Timeout) => panic!("commit_thread: recv_timeout reached - channel stall detected (>1s lull)"),
                Err(RecvTimeoutError::Disconnected) => if i == 0 {break 'outer} else {break 'inner},
            }
        }

        let mut txn = client.transaction()?;

        {
            let mut scpy = txn.copy_in(COPY_STAR)?;
            for (star, _) in &batch {
                scpy.write_all(star.as_bytes()).expect("writing to scpy failed");
            }
            scpy.finish()?;
        }

        {
            let mut pcpy = txn.copy_in(COPY_PLANET)?;
            for (_, planet) in &batch {
                pcpy.write_all(planet.as_bytes()).expect("writing to pcpy failed");
            }
            pcpy.finish()?;
        }

        txn.commit()?;
        COMMITTED_SEEDS.fetch_add(batch.len() as i32, std::sync::atomic::Ordering::SeqCst);
    }
    log_info!("Writer thread terminated");
    Ok(())
}
pub fn writer_sink(rec: Receiver<(String, String)>) -> Result<()> {
    loop {
        match rec.recv_timeout(Duration::new(1, 0)) {
            Ok(_) => {},
            Err(RecvTimeoutError::Timeout) => panic!("writer_sink: recv_timeout reached - channel stall detected (>1s lull)"),
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    Ok(())
}
