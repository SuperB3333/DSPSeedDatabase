mod macros;
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
    io::{Write, stdout},
    thread,
    sync::atomic::{
        AtomicI32
    }
};
use crossterm::ExecutableCommand;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use crate::{
    misc::split_chunks,
    metrics::write_metrics,
    threads::*
};
use crate::checkpoint::write_checkpoints;

const STAR_COUNT: usize = 64;
const REC_MULTIPLIER: f32 = 1.0;

const MAX_WORKERS: usize = 32;

static COMMITTED_SEEDS: AtomicI32 = AtomicI32::new(0);
static PROGRESS_WORKERS: [AtomicI32; MAX_WORKERS] = [const { AtomicI32::new(0) }; MAX_WORKERS];

lazy_static! {
    static ref START_SEED: i32 = env_int!("START_SEED", 0);
    static ref END_SEED: i32 = env_int!("END_SEED", 10_000);
    static ref WORKER_THREADS: i32 = env_int!("WORKER_THREADS", 8);
    static ref WRITER_THREADS: i32 = env_int!("WRITER_THREADS", 4);
    static ref COMMIT_COUNT: i32 = env_int!("COMMIT_COUNT", 1000);
    static ref CHANNEL_SIZE: i32 = env_int!("CHANNEL_SIZE", 1000);
    static ref CHECKPOINT_FILE: String = env_str!("CHECKPOINT_FILE", "checkpoints.txt");
    static ref BENCHMARK: bool = env_int!("BENCHMARK", 0) == 1;

    static ref DB_STR: String = {
        let user = env_str!("PG_USER", "postgres");
        let pass = env_str!("PG_PASS", "rootpassword");
        let netloc = env_str!("PG_NETLOC", "localhost");
        let port = env_str!("PG_PORT", "5432");
        let db_name = env_str!("PG_DBNAME", "dsp");
        format!("postgres://{user}:{pass}@{netloc}:{port}/{db_name}?sslmode=disable")
    };

    static ref MAX_BUFFER: i32 = *CHANNEL_SIZE + *COMMIT_COUNT * *WORKER_THREADS;
}
fn main() {
    assert!(*START_SEED < *END_SEED);
    assert!(*WORKER_THREADS < *END_SEED);
    assert!(*WORKER_THREADS < MAX_WORKERS as i32);


    let start = Instant::now();

    // Prepare thread resources
    let all_seeds = *START_SEED..*END_SEED;
    let workloads = split_chunks(all_seeds, *WORKER_THREADS);
    let (entry_sender, entry_reciever): (Sender<(String, String)>, Receiver<(String, String)>) = bounded(*CHANNEL_SIZE as usize);

    let mut work_handles = vec![];
    let mut commit_handles = vec![];
    // Launch worker threads
    for (id, work) in workloads.iter().enumerate() {
        let thread_sender = entry_sender.clone();
        let thread_work = work.clone();
        work_handles.push(thread::spawn(move || {
            worker_thread(thread_work, thread_sender, id)
        }))
    }
    if !*BENCHMARK {
        let conf = ((*DB_STR).clone(), *COMMIT_COUNT);
        // Launch database threads
        for _ in 0..*WRITER_THREADS {
            let thread_receiver = entry_reciever.clone();
            let thread_conf = conf.clone();
            commit_handles.push(thread::spawn(move || {
                commit_thread(thread_receiver, thread_conf)
            }))
        }
    }
    else {
        for _ in 0..*WRITER_THREADS {
            let thread_receiver = entry_reciever.clone();
            commit_handles.push(thread::spawn(move || {
                writer_sink(thread_receiver)
            }))
        }
    }
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen).unwrap();
    stdout.execute(crossterm::cursor::Hide).unwrap();

    loop {
        if !*BENCHMARK {
            write_checkpoints().unwrap();
        }

        write_metrics(-1.0, *END_SEED - *START_SEED, entry_reciever.len() as i32).unwrap(); //todo implement seeds/sec (arg 1)
        thread::sleep(Duration::from_millis(100));
        if work_handles.iter().all(|i| i.is_finished()) { break }
    }

    stdout.execute(crossterm::cursor::Show).unwrap();
    stdout.execute(LeaveAlternateScreen).unwrap();
    // Wait for threads to finish
    for handle in work_handles {
        handle.join().unwrap().unwrap(); // wait for all workers to finish
    }
    drop(entry_sender); // close the channel so recv() returns Err instead of blocking
    for handle in commit_handles {
        handle.join().unwrap().unwrap(); // wait for senders to finish
    }

    let elapsed = start.elapsed();
    let per_second = (*END_SEED - *START_SEED) as f32 / elapsed.as_secs() as f32;
    println!("seeds/sec: {:?}", per_second);
}