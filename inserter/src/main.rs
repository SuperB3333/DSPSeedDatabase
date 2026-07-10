mod algorithm;
mod generate_csv;
mod misc;
mod metrics;
mod logging;
mod threads;
mod checkpoint;

use crossbeam_channel::{bounded, Receiver, Sender};
use lazy_static::lazy_static;
use std::{
    time::{Duration, Instant},
    io::stdout,
    thread,
    sync::atomic::{AtomicI32, Ordering}
};
use crossterm::ExecutableCommand;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::tty::IsTty;
use crate::{
    metrics::write_metrics,
    threads::*
};
use crate::checkpoint::{load_workloads, write_checkpoints};


const MAIN_INTERVAL: f32 = 0.1; // in seconds
const STAR_COUNT: usize = 64;
const REC_MULTIPLIER: f32 = 1.0;

// The maximum amount of workers the binary supports. 32 should be way higher than what is reasonable on most machines
const MAX_WORKERS: usize = 32;

static COMMITTED_SEEDS: AtomicI32 = AtomicI32::new(0);
static PROGRESS_WORKERS: [AtomicI32; MAX_WORKERS] = [const { AtomicI32::new(0) }; MAX_WORKERS];

// Global configurations: Lazy values, read on first use
lazy_static! {
    static ref START_SEED: i32 = env_int!("START_SEED", 0);
    static ref END_SEED: i32 = env_int!("END_SEED", 10_000);
    static ref WORKER_THREADS: i32 = env_int!("WORKER_THREADS", 8);
    static ref WRITER_THREADS: i32 = env_int!("WRITER_THREADS", 4);
    static ref COMMIT_COUNT: usize = env_int!("COMMIT_COUNT", 1000) as usize;
    static ref CHANNEL_SIZE: usize = env_int!("CHANNEL_SIZE", 1000) as usize;
    static ref CHECKPOINT_FILE: String = env_str!("CHECKPOINT_FILE", "checkpoints.txt");
    static ref BENCHMARK: bool = env_int!("BENCHMARK", 0) == 1;

    static ref TUI: bool = crossterm::ansi_support::supports_ansi() && stdout().is_tty() && env_int!("NO_TUI", 0) != 1;

    static ref DB_STR: String = {
        let user = env_str!("PG_USER", "postgres");
        let pass = env_str!("PG_PASS", "rootpassword");
        let netloc = env_str!("PG_NETLOC", "localhost");
        let port = env_str!("PG_PORT", "5432");
        let db_name = env_str!("PG_DBNAME", "dsp");
        format!("postgres://{user}:{pass}@{netloc}:{port}/{db_name}?sslmode=disable")
    };

    static ref MAX_BUFFER: usize = *CHANNEL_SIZE + *COMMIT_COUNT * *WORKER_THREADS as usize;
}
fn main() {
    assert!(*START_SEED < *END_SEED, "START_SEED is lower than END_SEED");
    assert!(*WORKER_THREADS < (*END_SEED - *START_SEED), "More worker threads than seeds to process");
    assert!(*WORKER_THREADS < MAX_WORKERS as i32, "More worker threads than the binary allows! Try to compile with MAX_WORKERS set higher.");

    // capture start time for performance evaluation
    let start = Instant::now();

    // Prepare thread resources
    log_info!("Loading workloads...");
    let workloads = load_workloads();
    let (entry_sender, entry_reciever): (Sender<(String, String)>, Receiver<(String, String)>) = bounded(*CHANNEL_SIZE);

    let mut work_handles = vec![];
    let mut commit_handles = vec![];
    log_info!("Starting worker threads...");
    // Launch worker threads
    for (id, work) in workloads.iter().enumerate() {
        let thread_sender = entry_sender.clone();
        let thread_work = work.clone();
        work_handles.push(
            thread::Builder::new()
                .name(format!("worker_{}", id))
                .spawn(move || {
                    worker_thread(thread_work, thread_sender, id)
                })
                .expect(format!("Failed to spawn worker thread {}", id).as_str())
        );
    }
    log_info!("Starting writer threads...");
    if !*BENCHMARK {
        // Launch database threads
        for id in 0..*WRITER_THREADS {
            let thread_receiver = entry_reciever.clone();
            commit_handles.push(
                thread::Builder::new()
                    .name(format!("writer_{}", id))
                    .spawn(move || {
                        commit_thread(thread_receiver)
                    })
                    .expect(format!("Failed to spawn writer thread {}", id).as_str())
            );
        }
    }
    else {
        for _ in 0..*WRITER_THREADS {
            // launch dummy db threads that will void all results
            let thread_receiver = entry_reciever.clone();
            commit_handles.push(thread::spawn(move || {
                writer_sink(thread_receiver)
            }))
        }
    }

    log_info!("Starting main thread loop");
    // Main thread takes checkpoints and displays metrics to the terminal
    if *TUI {
        let mut stdout = stdout();
        stdout.execute(EnterAlternateScreen).expect("Failed to customize terminal. Consider setting NO_TUI to 1");
        stdout.execute(crossterm::cursor::Hide).expect("Failed to customize terminal. Consider setting NO_TUI to 1");
    }
    let mut last_progress: Vec<i32> = vec![];
    loop {
        if !*BENCHMARK {
            write_checkpoints().expect("Failed to read checkpoint file. Directory might not exist or permission is missing");
        }
        let cur_progress: Vec<i32> = PROGRESS_WORKERS.iter().map(|x| x.load(Ordering::Relaxed)).collect();
        let advanced = cur_progress.iter().zip(last_progress.iter()).map(|(cur, last)| (last - cur) as f32 / MAIN_INTERVAL);
        let seeds_sec = advanced.len() as f32 / advanced.sum::<f32>();

        last_progress = cur_progress.clone();


        if *TUI {
            write_metrics(seeds_sec, *END_SEED - *START_SEED, entry_reciever.len() as i32).expect("Failed to customize terminal. Consider setting NO_TUI to 1");
        }

        thread::sleep(Duration::from_millis(1000 * MAIN_INTERVAL as u64));
        if work_handles.iter().all(|i| i.is_finished()) {
            log_info!("All workers have finished!");
            break;
        }
    }
    if *TUI {
        let mut stdout = stdout();
        stdout.execute(crossterm::cursor::Show).expect("Failed to customize terminal. Consider setting NO_TUI to 1");
        stdout.execute(LeaveAlternateScreen).expect("Failed to customize terminal. Consider setting NO_TUI to 1");
    }
    // Wait for threads to finish
    for handle in work_handles {
        handle.join().expect("Error while joining writer threads").unwrap(); // wait for all workers to finish
    }
    drop(entry_sender); // close the channel so recv() returns Err instead of blocking
    log_info!("Waiting for writer threads to automatically shut down");
    for handle in commit_handles {
        handle.join().expect("Error while joining writer threads").unwrap(); // wait for senders to finish
    }

    let elapsed = start.elapsed();
    let per_second = (*END_SEED - *START_SEED) as f32 / elapsed.as_secs() as f32;
    println!("seeds/sec: {:?}", per_second);
}