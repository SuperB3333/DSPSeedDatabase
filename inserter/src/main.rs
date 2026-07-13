mod algorithm;
mod checkpoint;
mod generate_csv;
mod logging;
mod metrics;
mod misc;
mod threads;

use crate::checkpoint::{load_workloads, write_checkpoints};
use crate::misc::check_db_connection;
use crate::{metrics::write_metrics, threads::*};
use anyhow::{anyhow, Context, Result};
use crossbeam_channel::{bounded, Receiver, Sender};
use crossterm::{
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
    ExecutableCommand
};
use lazy_static::lazy_static;
use std::{
    io::stdout,
    process::ExitCode,
    sync::atomic::{AtomicI32, Ordering},
    thread,
    time::{Duration, Instant},
};


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
    static ref PRINT_CONFIG: bool = env_int!("PRINT_CONFIG", 0) == 1;

    static ref TUI: bool = supports_ansi() && stdout().is_tty() && env_int!("NO_TUI", 0) != 1;

    static ref PG_USER: String = env_str!("PG_USER", "postgres");
    static ref PG_PASS: String = env_str!("PG_PASS", "rootpassword");
    static ref PG_NETLOC: String = env_str!("PG_NETLOC", "localhost");
    static ref PG_PORT: String = env_str!("PG_PORT", "5432");
    static ref PG_DBNAME: String = env_str!("PG_DBNAME", "dsp");

    static ref DB_STR: String = {
        format!(
            "postgres://{}:{}@{}:{}/{}?sslmode=disable",
            PG_USER.as_str(),
            PG_PASS.as_str(),
            PG_NETLOC.as_str(),
            PG_PORT.as_str(),
            PG_DBNAME.as_str()
        )
    };

    static ref MAX_BUFFER: usize = *CHANNEL_SIZE + *COMMIT_COUNT * *WORKER_THREADS as usize;
}

#[cfg(windows)]
fn supports_ansi() -> bool { crossterm::ansi_support::supports_ansi() }
#[cfg(not(windows))]
fn supports_ansi() -> bool { true }

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            log_error!("{:#}", err);
            ExitCode::FAILURE
        }
    }
}

fn validate_config() -> Result<()> {
    if *WORKER_THREADS <= 0 {
        error_return!("WORKER_THREADS must be greater than 0");
    }
    if *WRITER_THREADS <= 0 {
        error_return!("WRITER_THREADS must be greater than 0");
    }
    if *COMMIT_COUNT == 0 {
        error_return!("COMMIT_COUNT must be greater than 0");
    }
    Ok(())
}

fn print_effective_config() {
    println!("config.start_seed={}", *START_SEED);
    println!("config.end_seed={}", *END_SEED);
    println!("config.seed_count={}", *END_SEED - *START_SEED);
    println!("config.worker_threads={}", *WORKER_THREADS);
    println!("config.writer_threads={}", *WRITER_THREADS);
    println!("config.commit_count={}", *COMMIT_COUNT);
    println!("config.channel_size={}", *CHANNEL_SIZE);
    println!("config.benchmark={}", *BENCHMARK);
    println!("config.checkpoint_file={}", *CHECKPOINT_FILE);
    println!("config.database.user={}", *PG_USER);
    println!("config.database.host={}", *PG_NETLOC);
    println!("config.database.port={}", *PG_PORT);
    println!("config.database.name={}", *PG_DBNAME);
    println!("config.database.password=redacted");
    println!("config.log_level={}", std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()));
    println!("config.print_config=true");
}

fn run() -> Result<()> {
    validate_config()?;
    assert!(*START_SEED < *END_SEED, "START_SEED is lower than END_SEED");
    assert!(*WORKER_THREADS < (*END_SEED - *START_SEED), "More worker threads than seeds to process");
    assert!(*WORKER_THREADS < MAX_WORKERS as i32, "More worker threads than the binary allows! Try to compile with MAX_WORKERS set higher.");

    if *PRINT_CONFIG {
        print_effective_config();
        return Ok(());
    }

    // capture start time for performance evaluation
    let start = Instant::now();

    if *BENCHMARK {
        log_info!("Benchmark mode enabled; database writes are disabled");
    }

    if !check_db_connection() {
        if *BENCHMARK {
            log_warn!("Database connection failed, continuing because BENCHMARK=1");
        } else {
            error_return!("Database connection failed and BENCHMARK is not enabled; exiting");
        }
    }

    // Prepare thread resources
    log_info!("Loading workloads...");
    let workloads = load_workloads().context("failed to load workloads")?;
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
                .spawn(move || worker_thread(thread_work, thread_sender, id))
                .with_context(|| format!("failed to spawn worker thread {}", id))?,
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
                    .spawn(move || commit_thread(thread_receiver))
                    .with_context(|| format!("failed to spawn writer thread {}", id))?,
            );
        }
    } else {
        for _ in 0..*WRITER_THREADS {
            // launch dummy db threads that will void all results
            let thread_receiver = entry_reciever.clone();
            commit_handles.push(thread::spawn(move || writer_sink(thread_receiver)))
        }
    }

    log_info!("Starting main thread loop");
    // Main thread takes checkpoints and displays metrics to the terminal
    if *TUI {
        let mut stdout = stdout();
        stdout.execute(EnterAlternateScreen)?;
        stdout.execute(crossterm::cursor::Hide)?;
    }
    let mut last_progress: Vec<i32> = vec![];
    loop {
        if !*BENCHMARK {
            write_checkpoints().context("failed to write checkpoints")?;
        }
        let cur_progress: Vec<i32> = PROGRESS_WORKERS.iter().map(|x| x.load(Ordering::Relaxed)).collect();
        let advanced = cur_progress.iter().zip(last_progress.iter()).map(|(cur, last)| (last - cur) as f32 / MAIN_INTERVAL);
        let seeds_sec = advanced.len() as f32 / advanced.sum::<f32>();

        last_progress = cur_progress.clone();


        if *TUI {
            write_metrics(seeds_sec, *END_SEED - *START_SEED, entry_reciever.len() as i32)
                .map_err(|err| anyhow!("failed to write metrics: {}", err))?;
        }

        thread::sleep(Duration::from_millis(1000 * MAIN_INTERVAL as u64));
        if work_handles.iter().all(|i| i.is_finished()) {
            log_info!("All workers have finished!");
            break;
        }
    }
    if *TUI {
        let mut stdout = stdout();
        stdout.execute(crossterm::cursor::Show)?;
        stdout.execute(LeaveAlternateScreen)?;
    }
    // Wait for threads to finish
    for handle in work_handles {
        handle
            .join()
            .map_err(|_| anyhow!("worker thread panicked"))?
            .context("worker thread failed")?;
    }
    drop(entry_sender); // close the channel so recv() returns Err instead of blocking
    log_info!("Waiting for writer threads to automatically shut down");
    for handle in commit_handles {
        handle
            .join()
            .map_err(|_| anyhow!("writer thread panicked"))?
            .context("writer thread failed")?;
    }

    let elapsed = start.elapsed();
    let per_second = (*END_SEED - *START_SEED) as f32 / elapsed.as_secs() as f32;
    println!("seeds/sec: {:?}", per_second);
    Ok(())
}
