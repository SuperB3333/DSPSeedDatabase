use std::ops::Range;
use std::sync::atomic::Ordering::Relaxed;
use std::time::Duration;
use std::io::Write;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use postgres::{Client, NoTls};
use anyhow::Result;
use crate::generate_csv::{
    gen_formatted, COPY_FOOTER, COPY_HEADER_FIELDS, COPY_SIGNATURE,
};
use crate::{log_info, COMMITTED_SEEDS, COMMIT_COUNT, DB_STR, PROGRESS_WORKERS};
use crate::misc::{COPY_PLANET, COPY_STAR};

pub fn worker_thread(seeds: Range<i32>, send: Sender<(Vec<u8>, Vec<u8>)>, id: usize) -> Result<()> {
    for seed in seeds {
        let entry = gen_formatted(seed).expect("gen_formatted failed");
        send.send(entry)?;
        PROGRESS_WORKERS[id].fetch_add(1, Relaxed);

    }
    log_info!("Worker thread {} finished sucessfully", id);
    Ok(())
}
pub fn commit_thread(rec: Receiver<(Vec<u8>, Vec<u8>)>) -> Result<()> {
    let mut client = Client::connect(&*DB_STR.as_str(), NoTls)?;
    let mut batch: Vec<(Vec<u8>, Vec<u8>)> = Vec::with_capacity(*COMMIT_COUNT);

    loop {
        batch.clear();
        match rec.recv() {
            Ok(msg) => batch.push(msg),
            Err(_) => break,
        }

        let mut disconnected = false;
        for _ in 1..*COMMIT_COUNT {
            match rec.recv_timeout(Duration::new(1, 0)) {
                Ok(msg) => batch.push(msg),
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }

        let mut txn = client.transaction()?;

        {
            let mut scpy = txn.copy_in(COPY_STAR)?;
            scpy.write_all(COPY_SIGNATURE)?;
            scpy.write_all(COPY_HEADER_FIELDS)?;
            for (star, _) in &batch {
                scpy.write_all(star)?;
            }
            scpy.write_all(COPY_FOOTER)?;
            scpy.finish()?;
        }

        {
            let mut pcpy = txn.copy_in(COPY_PLANET)?;
            pcpy.write_all(COPY_SIGNATURE)?;
            pcpy.write_all(COPY_HEADER_FIELDS)?;
            for (_, planet) in &batch {
                pcpy.write_all(planet)?;
            }
            pcpy.write_all(COPY_FOOTER)?;
            pcpy.finish()?;
        }

        txn.commit()?;
        COMMITTED_SEEDS.fetch_add(batch.len() as i32, Relaxed);
        if disconnected {
            break;
        }
    }
    log_info!("Writer thread terminated");
    Ok(())
}
pub fn writer_sink(rec: Receiver<(Vec<u8>, Vec<u8>)>) -> Result<()> {
    while rec.recv().is_ok() {}
    Ok(())
}
