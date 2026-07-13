use crate::{
    BENCHMARK, CHANNEL_SIZE, COMMITTED_SEEDS, COMMIT_COUNT, WORKER_THREADS, WRITER_THREADS,
};
use anyhow::{Context, Result};
use crossbeam_channel::Receiver;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Condvar, LazyLock, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

static ENABLED: LazyLock<bool> = LazyLock::new(|| {
    !matches!(
        std::env::var("PROGRESS_LOG")
            .unwrap_or_else(|_| "1".to_string())
            .to_ascii_lowercase()
            .as_str(),
        "0" | "false" | "no" | "off"
    )
});

static WORKER_TIME_NS: AtomicU64 = AtomicU64::new(0);
static WORKERS_FINISHED: AtomicU64 = AtomicU64::new(0);
static ACTIVE_WORKERS: AtomicU64 = AtomicU64::new(0);
static DB_CONNECTION_NS: AtomicU64 = AtomicU64::new(0);
static DB_TIME_NS: AtomicU64 = AtomicU64::new(0);
static DB_WAIT_NS: AtomicU64 = AtomicU64::new(0);
static DB_BATCHES: AtomicU64 = AtomicU64::new(0);
static WRITERS_WAITING: AtomicU64 = AtomicU64::new(0);
static WRITERS_DB_ACTIVE: AtomicU64 = AtomicU64::new(0);

pub fn enabled() -> bool {
    *ENABLED
}

fn add_duration(counter: &AtomicU64, duration: Duration) {
    counter.fetch_add(duration.as_nanos().min(u64::MAX as u128) as u64, Relaxed);
}

pub fn record_worker(duration: Duration) {
    add_duration(&WORKER_TIME_NS, duration);
    WORKERS_FINISHED.fetch_add(1, Relaxed);
    ACTIVE_WORKERS.fetch_sub(1, Relaxed);
}

pub fn worker_started() {
    ACTIVE_WORKERS.fetch_add(1, Relaxed);
}

pub fn record_db_connection(duration: Duration) {
    add_duration(&DB_CONNECTION_NS, duration);
}

pub fn record_db_wait(duration: Duration) {
    add_duration(&DB_WAIT_NS, duration);
    WRITERS_WAITING.fetch_sub(1, Relaxed);
}

pub fn db_wait_started() {
    WRITERS_WAITING.fetch_add(1, Relaxed);
}

pub fn record_db_batch(duration: Duration) {
    add_duration(&DB_TIME_NS, duration);
    DB_BATCHES.fetch_add(1, Relaxed);
    WRITERS_DB_ACTIVE.fetch_sub(1, Relaxed);
}

pub fn db_batch_started() {
    WRITERS_DB_ACTIVE.fetch_add(1, Relaxed);
}

struct Snapshot {
    at: Instant,
    generated: u64,
    committed: u64,
}

struct ProcessStats {
    rss_kib: Option<u64>,
    peak_rss_kib: Option<u64>,
    read_bytes: Option<u64>,
    write_bytes: Option<u64>,
}

pub struct ProgressLogger {
    stop: Arc<(Mutex<bool>, Condvar)>,
    handle: Option<JoinHandle<()>>,
}

impl ProgressLogger {
    pub fn start(planned_seeds: u64, receiver: Receiver<(String, String)>) -> Result<Option<Self>> {
        if !enabled() {
            return Ok(None);
        }

        let interval_seconds = std::env::var("PROGRESS_LOG_INTERVAL_SECS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .context("PROGRESS_LOG_INTERVAL_SECS must be a positive integer")?;
        if interval_seconds == 0 {
            anyhow::bail!("PROGRESS_LOG_INTERVAL_SECS must be greater than 0");
        }
        let interval = Duration::from_secs(interval_seconds);
        eprintln!(
            "[progress] generation started: seeds={} workers={} writers={} commit_count={} interval={}s benchmark={}",
            planned_seeds,
            *WORKER_THREADS,
            *WRITER_THREADS,
            *COMMIT_COUNT,
            interval_seconds,
            *BENCHMARK
        );

        let stop = Arc::new((Mutex::new(false), Condvar::new()));
        let thread_stop = Arc::clone(&stop);
        let handle = thread::Builder::new()
            .name("progress_logger".to_string())
            .spawn(move || monitor(planned_seeds, receiver, interval, thread_stop))
            .context("failed to spawn progress logger")?;

        Ok(Some(Self {
            stop,
            handle: Some(handle),
        }))
    }

    pub fn finish(mut self) -> Result<()> {
        self.signal_stop();
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("progress logger panicked"))?;
        }
        Ok(())
    }

    fn signal_stop(&self) {
        let (lock, condition) = &*self.stop;
        if let Ok(mut stopped) = lock.lock() {
            *stopped = true;
            condition.notify_one();
        }
    }
}

impl Drop for ProgressLogger {
    fn drop(&mut self) {
        self.signal_stop();
    }
}

fn monitor(
    planned_seeds: u64,
    receiver: Receiver<(String, String)>,
    interval: Duration,
    stop: Arc<(Mutex<bool>, Condvar)>,
) {
    let started = Instant::now();
    let mut previous = Snapshot {
        at: started,
        generated: generated_seeds(),
        committed: committed_seeds(),
    };

    loop {
        let (lock, condition) = &*stop;
        let stopped = match lock.lock() {
            Ok(stopped) => stopped,
            Err(_) => return,
        };
        let (stopped, timeout) = match condition.wait_timeout(stopped, interval) {
            Ok(result) => result,
            Err(_) => return,
        };
        if *stopped {
            break;
        }
        if timeout.timed_out() {
            previous = report(started, previous, planned_seeds, receiver.len(), false);
        }
    }

    report(started, previous, planned_seeds, receiver.len(), true);
}

fn report(
    started: Instant,
    previous: Snapshot,
    planned_seeds: u64,
    queue: usize,
    complete: bool,
) -> Snapshot {
    let now = Instant::now();
    let generated = generated_seeds();
    let committed = committed_seeds();
    let elapsed = now.duration_since(started);
    let period = now
        .duration_since(previous.at)
        .as_secs_f64()
        .max(f64::EPSILON);
    let generated_rate = generated.saturating_sub(previous.generated) as f64 / period;
    let committed_rate = committed.saturating_sub(previous.committed) as f64 / period;
    let average_generated_rate = generated as f64 / elapsed.as_secs_f64().max(f64::EPSILON);
    let average_committed_rate = committed as f64 / elapsed.as_secs_f64().max(f64::EPSILON);
    let generated_percent = percent(generated, planned_seeds);
    let committed_percent = percent(committed, planned_seeds);
    let overall = if *BENCHMARK { generated } else { committed };
    let status = if complete { "complete" } else { "running" };
    let generation_time = estimated_generation_time(elapsed);

    eprintln!(
        "[progress] status={} elapsed={} progress={}/{} ({:.2}%) generated={}/{} ({:.2}%) committed={}/{} ({:.2}%) queue={}/{}",
        status,
        format_duration(elapsed),
        overall,
        planned_seeds,
        percent(overall, planned_seeds),
        generated,
        planned_seeds,
        generated_percent,
        committed,
        planned_seeds,
        committed_percent,
        queue,
        *CHANNEL_SIZE
    );
    eprintln!(
        "[progress] rates: generated_interval={:.3}/s generated_average={:.3}/s committed_interval={:.3}/s committed_average={:.3}/s",
        generated_rate,
        average_generated_rate,
        committed_rate,
        average_committed_rate
    );
    eprintln!(
        "[progress] timing: generation_worker={} db_connection={} db_active={} writer_wait={} workers_active={} workers_finished={}/{} writers_waiting={} writers_db_active={} db_batches={} dominant_thread_time={} likely_bottleneck={}",
        format_nanos(generation_time),
        format_nanos(DB_CONNECTION_NS.load(Relaxed)),
        format_nanos(DB_TIME_NS.load(Relaxed)),
        format_nanos(DB_WAIT_NS.load(Relaxed)),
        ACTIVE_WORKERS.load(Relaxed),
        WORKERS_FINISHED.load(Relaxed),
        *WORKER_THREADS,
        WRITERS_WAITING.load(Relaxed),
        WRITERS_DB_ACTIVE.load(Relaxed),
        DB_BATCHES.load(Relaxed),
        dominant_stage(generation_time),
        likely_bottleneck(generated, committed, planned_seeds, queue)
    );
    print_process_stats(read_process_stats());

    Snapshot {
        at: now,
        generated,
        committed,
    }
}

fn generated_seeds() -> u64 {
    crate::PROGRESS_WORKERS
        .iter()
        .take(*WORKER_THREADS as usize)
        .map(|progress| progress.load(Relaxed).max(0) as u64)
        .sum()
}

fn committed_seeds() -> u64 {
    COMMITTED_SEEDS.load(Relaxed).max(0) as u64
}

fn percent(value: u64, total: u64) -> f64 {
    if total == 0 {
        100.0
    } else {
        value.min(total) as f64 / total as f64 * 100.0
    }
}

fn estimated_generation_time(elapsed: Duration) -> u64 {
    let active_time = elapsed
        .as_nanos()
        .saturating_mul(ACTIVE_WORKERS.load(Relaxed) as u128)
        .min(u64::MAX as u128) as u64;
    WORKER_TIME_NS.load(Relaxed).saturating_add(active_time)
}

fn dominant_stage(generation_time: u64) -> &'static str {
    let stages = [
        ("generation", generation_time),
        ("database", DB_TIME_NS.load(Relaxed)),
        ("writer_wait", DB_WAIT_NS.load(Relaxed)),
    ];
    stages
        .into_iter()
        .max_by_key(|(_, duration)| *duration)
        .filter(|(_, duration)| *duration > 0)
        .map(|(stage, _)| stage)
        .unwrap_or("warming_up")
}

fn likely_bottleneck(
    generated: u64,
    committed: u64,
    planned_seeds: u64,
    queue: usize,
) -> &'static str {
    if *BENCHMARK {
        return "generation";
    }
    if generated >= planned_seeds && committed < planned_seeds {
        return "database_drain";
    }
    if *CHANNEL_SIZE > 0 && queue.saturating_mul(100) >= *CHANNEL_SIZE * 80 {
        return "database";
    }
    if *CHANNEL_SIZE == 0 || queue.saturating_mul(100) <= *CHANNEL_SIZE * 20 {
        return "generation";
    }
    "balanced_or_inconclusive"
}

fn format_nanos(nanos: u64) -> String {
    format_duration(Duration::from_nanos(nanos))
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs_f64();
    if seconds >= 60.0 {
        format!("{}m{:.3}s", duration.as_secs() / 60, seconds % 60.0)
    } else {
        format!("{:.3}s", seconds)
    }
}

fn read_process_stats() -> ProcessStats {
    let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
    let io = fs::read_to_string("/proc/self/io").unwrap_or_default();
    ProcessStats {
        rss_kib: parse_value(&status, "VmRSS:"),
        peak_rss_kib: parse_value(&status, "VmHWM:"),
        read_bytes: parse_value(&io, "read_bytes:"),
        write_bytes: parse_value(&io, "write_bytes:"),
    }
}

fn parse_value(contents: &str, key: &str) -> Option<u64> {
    contents
        .lines()
        .find(|line| line.starts_with(key))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse().ok())
}

fn print_process_stats(stats: ProcessStats) {
    eprintln!(
        "[progress] resources: rss_mib={} peak_rss_mib={} process_read_mib={} process_write_mib={}",
        format_mib(stats.rss_kib.map(|value| value * 1024)),
        format_mib(stats.peak_rss_kib.map(|value| value * 1024)),
        format_mib(stats.read_bytes),
        format_mib(stats.write_bytes)
    );
}

fn format_mib(bytes: Option<u64>) -> String {
    bytes
        .map(|value| format!("{:.3}", value as f64 / 1_048_576.0))
        .unwrap_or_else(|| "n/a".to_string())
}

#[cfg(test)]
mod tests {
    use super::{format_duration, parse_value, percent};
    use std::time::Duration;

    #[test]
    fn formats_short_and_long_durations() {
        assert_eq!(format_duration(Duration::from_millis(1250)), "1.250s");
        assert_eq!(format_duration(Duration::from_millis(61_250)), "1m1.250s");
    }

    #[test]
    fn percentage_is_bounded() {
        assert_eq!(percent(50, 100), 50.0);
        assert_eq!(percent(200, 100), 100.0);
    }

    #[test]
    fn parses_proc_values() {
        assert_eq!(parse_value("VmRSS:\t2048 kB\n", "VmRSS:"), Some(2048));
        assert_eq!(parse_value("read_bytes: 4096\n", "read_bytes:"), Some(4096));
    }
}
