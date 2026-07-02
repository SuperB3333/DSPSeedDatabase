mod macros;
mod algorithm;
mod generate_csv;
mod misc;
mod metrics;

use postgres::{Client, NoTls};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::{
    time::{Duration, Instant},
    io::Write,
    ops::Range,
    thread
};
use std::fs::OpenOptions;
use std::io::stdout;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering::{Relaxed};
use crossterm::ExecutableCommand;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use generate_csv::gen_formatted;
use macros::env_int;
use misc::{split_chunks, COPY_PLANET, COPY_STAR, get_db_str};
use crate::macros::env_str;
use crate::metrics::write_metrics;

const STAR_COUNT: usize = 64;
const REC_MULTIPLIER: f32 = 1.0;

static COMMITTED_SEEDS: AtomicI32 = AtomicI32::new(0);

static PROGRESS_WORKERS: [AtomicI32; 32] = [const { AtomicI32::new(0) }; 32];

fn worker_thread(seeds: Range<i32>, send: Sender<(String, String)>, id: usize) {
    for seed in seeds {
        let entry = gen_formatted(seed, STAR_COUNT, REC_MULTIPLIER).expect("gen_formatted failed");
        send.send(entry).unwrap();
        PROGRESS_WORKERS[id].fetch_add(1, Relaxed);

    }
}
fn commit_thread(rec: Receiver<(String, String)>, config: (String, i32)) {
    let mut client = Client::connect(config.0.as_str(), NoTls).unwrap();
    let commit_size = config.1 as usize;

    loop {
        let mut batch: Vec<(String, String)> = Vec::with_capacity(commit_size);
        for _ in 0..commit_size {
            match rec.recv_timeout(Duration::new(1, 0)) {
                Ok(msg) => batch.push(msg),
                Err(_) => break,
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
fn main() {
    // Retrieve Config
    let start_seed = env_int!("START_SEED", 0);
    let end_seed = env_int!("END_SEED", 10_000);
    let worker_count = env_int!("WORKER_THREADS", 8);
    let writer_count = env_int!("WRITER_THREADS", 4);
    let commit_count = env_int!("COMMIT_COUNT", 1000);
    let channel_size = env_int!("CHANNEL_SIZE", 1000);

    let checkpoint_file = env_str!("CHECKPOINT_FILE", "checkpoints.txt");

    let conf = (get_db_str(), commit_count);

    assert!(start_seed < end_seed);
    assert!(worker_count < end_seed);
    assert!(worker_count < 32);


    let start = Instant::now();

    // Prepare thread resources
    let all_seeds = start_seed..end_seed;
    let workloads = split_chunks(all_seeds, worker_count);
    let (entry_sender, entry_reciever): (Sender<(String, String)>, Receiver<(String, String)>) = bounded(channel_size as usize);

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
    // Launch database threads
    for _ in 0..writer_count {
        let thread_receiver = entry_reciever.clone();
        let thread_conf = conf.clone();
        commit_handles.push(thread::spawn(move || {
            commit_thread(thread_receiver, thread_conf);
        }))
    }
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen).unwrap();
    stdout.execute(crossterm::cursor::Hide).unwrap();

    loop {
        let max_buffer = channel_size + commit_count * writer_count;
        let mut cfile = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&checkpoint_file)
            .unwrap();
        PROGRESS_WORKERS
            .iter()
            .map(|i| i.load(Relaxed) - max_buffer)
            .for_each(|x| {
                writeln!(cfile, "{}", x).unwrap();
            });





        write_metrics(-1.0, end_seed - start_seed, entry_reciever.len() as i32).unwrap();
        thread::sleep(Duration::from_millis(100));
        if work_handles.iter().all(|i| i.is_finished()) { break }
    }

    stdout.execute(crossterm::cursor::Show).unwrap();
    stdout.execute(LeaveAlternateScreen).unwrap();
    // Wait for threads to finish
    for handle in work_handles {
        handle.join().unwrap(); // wait for all workers to finish
    }
    drop(entry_sender); // close the channel so recv() returns Err instead of blocking
    for handle in commit_handles {
        handle.join().unwrap(); // wait for senders to finish
    }

    let elapsed = start.elapsed();
    let per_second = (end_seed - start_seed) as f32 / elapsed.as_secs() as f32;
    println!("seeds/sec: {:?}", per_second);
}