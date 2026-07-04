mod macros;
mod algorithm;
mod generate_csv;
mod misc;
mod metrics;
mod logging;

use postgres::{Client, NoTls};
use crossbeam_channel::{bounded, Receiver, Sender, RecvTimeoutError};
use std::{
    time::{Duration, Instant},
    io::Write,
    ops::Range,
    thread
};
use std::io::stdout;
use std::io::IsTerminal;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::{Relaxed};
use crossterm::ExecutableCommand;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use generate_csv::gen_formatted;
use macros::env_int;
use misc::{split_chunks, write_checkpoint_atomic, read_checkpoint, Checkpoint, WorkerCheckpoint, COPY_PLANET, COPY_STAR, get_db_str};
use crate::macros::env_str;
use crate::metrics::{write_metrics, log_progress};
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};

const STAR_COUNT: usize = 64;
const REC_MULTIPLIER: f32 = 1.0;

static COMMITTED_SEEDS: AtomicI32 = AtomicI32::new(0);

static BENCH_BYTES: AtomicU64 = AtomicU64::new(0);

static PROGRESS_WORKERS: [AtomicI32; 32] = [const { AtomicI32::new(0) }; 32];

fn worker_thread(seeds: Range<i32>, send: Sender<(String, String)>, id: usize) {
    for seed in seeds {
        let entry = gen_formatted(seed, STAR_COUNT, REC_MULTIPLIER).expect("gen_formatted failed");
        send.send(entry).unwrap();
        PROGRESS_WORKERS[id].fetch_add(1, Relaxed);

    }
}
fn commit_thread(rec: Receiver<(String, String)>, config: (String, i32)) {
    let mut client = match Client::connect(config.0.as_str(), NoTls) {
        Ok(c) => {
            log_debug!("commit thread connected to database");
            c
        }
        Err(e) => {
            log_error!("commit thread failed to connect to database: {}", e);
            panic!("commit_thread: database connection failed: {}", e);
        }
    };
    let commit_size = config.1 as usize;

    loop {
        let mut batch: Vec<(String, String)> = Vec::with_capacity(commit_size);
        for _ in 0..commit_size {
            match rec.recv_timeout(Duration::new(1, 0)) {
                Ok(msg) => batch.push(msg),
                Err(RecvTimeoutError::Timeout) =>
                    panic!("commit_thread: recv_timeout reached - channel stall detected (>1s lull)"),
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }

        if batch.is_empty() {
            break;
        }

        let mut txn = client.transaction().unwrap();

        {
            let mut scpy = txn.copy_in(COPY_STAR).unwrap();
            for (star, _) in &batch {
                scpy.write_all(star.as_bytes()).expect("writing to scpy failed");
            }
            scpy.finish().unwrap();
        }

        {
            let mut pcpy = txn.copy_in(COPY_PLANET).unwrap();
            for (_, planet) in &batch {
                pcpy.write_all(planet.as_bytes()).expect("writing to pcpy failed");
            }
            pcpy.finish().unwrap();
        }

        txn.commit().unwrap();
        COMMITTED_SEEDS.fetch_add(batch.len() as i32, std::sync::atomic::Ordering::SeqCst);
    }
}
// Benchmark-only channel consumer: drains the exact same producer stream as the
// writers but touches no Postgres and no checkpoint. Isolates pure-generation
// CPU throughput. Plain recv() exits on Disconnected once main drops the sender
// — identical shutdown path to commit_thread. COMMITTED_SEEDS increments keep the
// Order-03 TUI/progress reporting working unmodified.
fn bench_sink_thread(rec: Receiver<(String, String)>) {
    while let Ok((s, p)) = rec.recv() {
        BENCH_BYTES.fetch_add((s.len() + p.len()) as u64, Relaxed);
        COMMITTED_SEEDS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}
fn main() {
    logging::init_from_env();

    // Retrieve Config
    let start_seed = env_int!("START_SEED", 0);
    let end_seed = env_int!("END_SEED", 10_000);
    let worker_count = env_int!("WORKER_THREADS", 8);
    let writer_count = env_int!("WRITER_THREADS", 4);
    let commit_count = env_int!("COMMIT_COUNT", 1000);
    let channel_size = env_int!("CHANNEL_SIZE", 1000);

    let checkpoint_file = env_str!("CHECKPOINT_FILE", "checkpoints.txt");

    // Benchmark mode: pure-generation throughput. Disables Postgres and all
    // checkpoint state I/O so a bench run can never corrupt real-run state.
    let benchmark = env_str!("BENCHMARK", "0") == "1";
    if benchmark {
        log_info!("benchmark mode: DB and checkpointing disabled");
    }

    // In normal mode only, build the DB connection config. In benchmark mode we
    // never call get_db_str() so no PG credentials/connection are touched.
    let conf: Option<(String, i32)> = if benchmark {
        None
    } else {
        Some((get_db_str(), commit_count))
    };

    log_info!(
        "config: seeds {}..{}, workers={}, writers={}, commit_count={}, channel_size={}",
        start_seed, end_seed, worker_count, writer_count, commit_count, channel_size
    );
    log_debug!("checkpoint file: {}", checkpoint_file);

    assert!(start_seed < end_seed, "START_SEED ({}) must be less than END_SEED ({})", start_seed, end_seed);
    assert!(worker_count < end_seed, "WORKER_THREADS ({}) must be less than END_SEED ({})", worker_count, end_seed);
    assert!(worker_count < 32, "WORKER_THREADS ({}) must be at most 31, got {}", worker_count, worker_count);

    // Conservative in-flight bound: everything below (generated - max_buffer)
    // is guaranteed committed, since at most `max_buffer` seeds can be buffered
    // in the channel or the writers' pending batches at any instant.
    let max_buffer = channel_size + commit_count * writer_count;

    // Resume from checkpoint if available. `workloads` are the per-worker
    // ranges actually fed to worker threads below.
    let workloads: Vec<Range<i32>>;
    if !benchmark {
    match read_checkpoint(&checkpoint_file) {
        Ok(Some(cp)) => {
            // Header must match current config exactly, else we would either
            // skip or duplicate seeds. Never guess — bail out loudly.
            if cp.start_seed != start_seed
                || cp.end_seed != end_seed
                || cp.worker_count != worker_count
            {
                log_error!(
                    "checkpoint mismatch: file has (start={}, end={}, workers={}) but config is (start={}, end={}, workers={}); delete {} or restore config",
                    cp.start_seed, cp.end_seed, cp.worker_count,
                    start_seed, end_seed, worker_count,
                    checkpoint_file
                );
                std::process::exit(1);
            }

            // All work already done?
            if cp.workers.iter().all(|w| w.watermark >= w.end) {
                log_info!("all work done");
                return;
            }

            // Per-worker resume range = watermark..chunk_end (reuse recorded chunks).
            let resume: Vec<Range<i32>> = cp
                .workers
                .iter()
                .map(|w| w.watermark.max(w.start)..w.end)
                .collect();

            // Duplicate purge (Invariant B): delete the conservatively-rewound
            // window for each worker before spawning, so a resumed COPY never
            // collides with an already-committed stars.id PK.
            //   star_id = seed*100 + index, index < 64, stride 100.
            log_info!("resuming from checkpoint; purging rewound windows before restart");
            {
                let mut purge_client = match Client::connect(conf.as_ref().unwrap().0.as_str(), NoTls) {
                    Ok(c) => c,
                    Err(e) => {
                        log_error!("resume purge: failed to connect to database: {}", e);
                        panic!("resume purge: database connection failed: {}", e);
                    }
                };
                for w in &cp.workers {
                    if w.watermark >= w.end {
                        continue; // this chunk is fully done, nothing to redo/purge
                    }
                    // star_id / id columns are INT (i32); seed*100 stays well
                    // within i32 for all supported seed ranges.
                    let lo: i32 = w.watermark.max(w.start) * 100;
                    let hi: i32 = w.end * 100;
                    purge_client
                        .execute(
                            "DELETE FROM planets WHERE star_id >= $1 AND star_id < $2",
                            &[&lo, &hi],
                        )
                        .expect("resume purge: DELETE FROM planets failed");
                    purge_client
                        .execute(
                            "DELETE FROM stars WHERE id >= $1 AND id < $2",
                            &[&lo, &hi],
                        )
                        .expect("resume purge: DELETE FROM stars failed");
                }
            }

            workloads = resume;
        }
        Ok(None) => {
            log_debug!("no checkpoint file found, starting fresh");
            workloads = split_chunks(start_seed..end_seed, worker_count);
        }
        Err(e) => {
            log_debug!("checkpoint unparseable ({}), starting fresh", e);
            workloads = split_chunks(start_seed..end_seed, worker_count);
        }
    }
    } else {
        // Benchmark mode: never read the checkpoint file. Always start fresh
        // across the full seed range.
        workloads = split_chunks(start_seed..end_seed, worker_count);
    }


    let start = Instant::now();
    let (entry_sender, entry_reciever): (Sender<(String, String)>, Receiver<(String, String)>) = bounded(channel_size as usize);

    log_info!("launching {} worker threads and {} writer threads", worker_count, writer_count);

    let mut work_handles = vec![];
    let mut commit_handles = vec![];
    // Launch worker threads
    for (id, work) in workloads.iter().enumerate() {
        let thread_sender = entry_sender.clone();
        let thread_work = work.clone();
        work_handles.push(thread::spawn(move || {
            worker_thread(thread_work, thread_sender, id);
        }))
    }
    // Launch channel consumers. Same count in both modes (harmless extra sinks
    // keep the loop structure identical). In benchmark mode we swap the consumer
    // for bench_sink_thread — the producer (worker_thread) path is untouched.
    for _ in 0..writer_count {
        let thread_receiver = entry_reciever.clone();
        if benchmark {
            commit_handles.push(thread::spawn(move || {
                bench_sink_thread(thread_receiver);
            }))
        } else {
            let thread_conf = conf.as_ref().unwrap().clone();
            commit_handles.push(thread::spawn(move || {
                commit_thread(thread_receiver, thread_conf);
            }))
        }
    }
    // TUI is only enabled on an interactive stdout and when not explicitly disabled.
    let tui = std::io::stdout().is_terminal() && env_str!("NO_TUI", "0") != "1";

    let mut stdout = stdout();
    if tui {
        enable_raw_mode().unwrap();
        stdout.execute(EnterAlternateScreen).unwrap();
        stdout.execute(crossterm::cursor::Hide).unwrap();
    }

    // Build the current checkpoint snapshot from live per-worker progress.
    // For worker i with recorded chunk cs..ce and `generated` seeds produced:
    //   watermark = (cs + generated - max_buffer).clamp(cs, ce)
    // Conservative: at most `max_buffer` produced seeds may still be in-flight,
    // so everything strictly below `watermark` is already committed.
    let snapshot = |force_complete: bool| -> Checkpoint {
        let workers: Vec<WorkerCheckpoint> = workloads
            .iter()
            .enumerate()
            .map(|(i, chunk)| {
                let cs = chunk.start;
                let ce = chunk.end;
                let watermark = if force_complete {
                    ce
                } else {
                    let generated = PROGRESS_WORKERS[i].load(Relaxed);
                    (cs + generated - max_buffer).clamp(cs, ce)
                };
                WorkerCheckpoint { start: cs, end: ce, watermark }
            })
            .collect();
        Checkpoint {
            start_seed,
            end_seed,
            worker_count,
            workers,
        }
    };

    // Cadence: UI ticks every 100ms; write a checkpoint every ~5s (tick % 50).
    let mut tick: u64 = 0;
    loop {
        // Order-02 periodic checkpoint write — skipped entirely in benchmark mode
        // (no checkpoint file is ever touched). Order-03 progress reporting below
        // still runs on the same cadence in both modes.
        if !benchmark && tick % 50 == 0 {
            let cp = snapshot(false);
            if let Err(e) = write_checkpoint_atomic(&checkpoint_file, &cp) {
                log_warn!("failed to write checkpoint '{}': {}", checkpoint_file, e);
            }
        }


        // Real seeds/sec from committed count over elapsed wall time.
        let sps = COMMITTED_SEEDS.load(Relaxed) as f32 / start.elapsed().as_secs_f32().max(1e-6);
        let goal = end_seed - start_seed;
        let queue = entry_reciever.len() as i32;

        if tui {
            write_metrics(sps, goal, queue).unwrap();
        } else if tick % 50 == 0 {
            // stderr progress fallback on the same ~5s cadence; never when TUI is active.
            log_progress(sps, goal, queue);
        }

        thread::sleep(Duration::from_millis(100));
        tick += 1;
        if work_handles.iter().all(|i| i.is_finished()) { break }
    }

    if tui {
        stdout.execute(crossterm::cursor::Show).unwrap();
        stdout.execute(LeaveAlternateScreen).unwrap();
        disable_raw_mode().unwrap();
    }
    // Wait for threads to finish
    for handle in work_handles {
        handle.join().unwrap(); // wait for all workers to finish
    }
    drop(entry_sender); // close the channel so recv() returns Err instead of blocking
    for handle in commit_handles {
        handle.join().unwrap(); // wait for senders to finish
    }

    // Clean-completion checkpoint: all commit threads have joined, so everything
    // is committed. Record watermark == chunk_end for every worker so a later
    // run sees "all work done". Skipped in benchmark mode (no checkpoint I/O).
    if !benchmark {
        let final_cp = snapshot(true);
        if let Err(e) = write_checkpoint_atomic(&checkpoint_file, &final_cp) {
            log_warn!("failed to write final checkpoint '{}': {}", checkpoint_file, e);
        }
    }

    let elapsed = start.elapsed();
    let secs = elapsed.as_secs_f32();
    let per_second = if secs > 0.0 {
        (end_seed - start_seed) as f32 / secs
    } else {
        0.0
    };
    if benchmark {
        // Greppable benchmark result line. MB/MB-s derived from BENCH_BYTES, the
        // total generated star+planet payload drained by the sink threads.
        let total_bytes = BENCH_BYTES.load(Relaxed);
        let mb = total_bytes as f64 / (1024.0 * 1024.0);
        let mb_per_sec = if secs > 0.0 { mb / secs as f64 } else { 0.0 };
        log_info!(
            "benchmark: generated {} seeds in {:.2}s ({:.0} seeds/sec), {:.2} MB ({:.2} MB/s)",
            end_seed - start_seed,
            secs,
            per_second,
            mb,
            mb_per_sec
        );
    } else {
        log_info!(
            "done: generated {} seeds in {:.2}s ({:.0} seeds/sec)",
            end_seed - start_seed,
            secs,
            per_second
        );
    }
}