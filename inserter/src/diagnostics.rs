use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

static ENABLED: LazyLock<bool> = LazyLock::new(|| {
    matches!(
        std::env::var("DIAGNOSTICS")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
});

static GENERATED_SEEDS: AtomicU64 = AtomicU64::new(0);
static CSV_BYTES: AtomicU64 = AtomicU64::new(0);
static GENERATION_NS: AtomicU64 = AtomicU64::new(0);
static SEND_WAIT_NS: AtomicU64 = AtomicU64::new(0);
static CONNECTIONS: AtomicU64 = AtomicU64::new(0);
static CONNECTION_NS: AtomicU64 = AtomicU64::new(0);
static BATCH_RECEIVE_NS: AtomicU64 = AtomicU64::new(0);
static TRANSACTION_START_NS: AtomicU64 = AtomicU64::new(0);
static STAR_COPY_NS: AtomicU64 = AtomicU64::new(0);
static PLANET_COPY_NS: AtomicU64 = AtomicU64::new(0);
static COMMIT_NS: AtomicU64 = AtomicU64::new(0);
static BATCHES: AtomicU64 = AtomicU64::new(0);
static BATCHED_SEEDS: AtomicU64 = AtomicU64::new(0);
static MIN_BATCH_SIZE: AtomicU64 = AtomicU64::new(u64::MAX);
static MAX_BATCH_SIZE: AtomicU64 = AtomicU64::new(0);

pub fn enabled() -> bool {
    *ENABLED
}

pub struct ReportOnDrop(Option<Instant>);

impl Drop for ReportOnDrop {
    fn drop(&mut self) {
        if let Some(start) = self.0 {
            report(start.elapsed());
        }
    }
}

pub fn report_on_drop() -> ReportOnDrop {
    ReportOnDrop(enabled().then(Instant::now))
}

fn add_duration(counter: &AtomicU64, duration: Duration) {
    let nanos = duration.as_nanos().min(u64::MAX as u128) as u64;
    counter.fetch_add(nanos, Relaxed);
}

pub fn record_generation(duration: Duration, csv_bytes: usize) {
    GENERATED_SEEDS.fetch_add(1, Relaxed);
    CSV_BYTES.fetch_add(csv_bytes as u64, Relaxed);
    add_duration(&GENERATION_NS, duration);
}

pub fn record_send_wait(duration: Duration) {
    add_duration(&SEND_WAIT_NS, duration);
}

pub fn record_connection(duration: Duration) {
    CONNECTIONS.fetch_add(1, Relaxed);
    add_duration(&CONNECTION_NS, duration);
}

pub fn record_batch_receive(duration: Duration) {
    add_duration(&BATCH_RECEIVE_NS, duration);
}

pub fn record_transaction_start(duration: Duration) {
    add_duration(&TRANSACTION_START_NS, duration);
}

pub fn record_star_copy(duration: Duration) {
    add_duration(&STAR_COPY_NS, duration);
}

pub fn record_planet_copy(duration: Duration) {
    add_duration(&PLANET_COPY_NS, duration);
}

pub fn record_commit(duration: Duration, batch_size: usize) {
    let batch_size = batch_size as u64;
    add_duration(&COMMIT_NS, duration);
    BATCHES.fetch_add(1, Relaxed);
    BATCHED_SEEDS.fetch_add(batch_size, Relaxed);
    MIN_BATCH_SIZE.fetch_min(batch_size, Relaxed);
    MAX_BATCH_SIZE.fetch_max(batch_size, Relaxed);
}

fn seconds(counter: &AtomicU64) -> f64 {
    counter.load(Relaxed) as f64 / 1_000_000_000.0
}

fn report(elapsed: Duration) {
    let elapsed_seconds = elapsed.as_secs_f64();
    let generated_seeds = GENERATED_SEEDS.load(Relaxed);
    let csv_bytes = CSV_BYTES.load(Relaxed);
    let batches = BATCHES.load(Relaxed);
    let batched_seeds = BATCHED_SEEDS.load(Relaxed);
    let min_batch_size = if batches == 0 {
        0
    } else {
        MIN_BATCH_SIZE.load(Relaxed)
    };
    let average_batch_size = if batches == 0 {
        0.0
    } else {
        batched_seeds as f64 / batches as f64
    };
    let average_generation_ms = if generated_seeds == 0 {
        0.0
    } else {
        seconds(&GENERATION_NS) * 1000.0 / generated_seeds as f64
    };
    let csv_mib_per_second = if elapsed_seconds == 0.0 {
        0.0
    } else {
        csv_bytes as f64 / 1_048_576.0 / elapsed_seconds
    };

    println!("diagnostics.elapsed_seconds={:.6}", elapsed_seconds);
    println!("diagnostics.generated_seeds={}", generated_seeds);
    println!("diagnostics.csv_bytes={}", csv_bytes);
    println!("diagnostics.csv_mib_per_second={:.6}", csv_mib_per_second);
    println!(
        "diagnostics.generation_aggregate_seconds={:.6}",
        seconds(&GENERATION_NS)
    );
    println!(
        "diagnostics.generation_average_ms={:.6}",
        average_generation_ms
    );
    println!(
        "diagnostics.channel_send_wait_aggregate_seconds={:.6}",
        seconds(&SEND_WAIT_NS)
    );
    println!(
        "diagnostics.writer_connections={}",
        CONNECTIONS.load(Relaxed)
    );
    println!(
        "diagnostics.writer_connection_aggregate_seconds={:.6}",
        seconds(&CONNECTION_NS)
    );
    println!(
        "diagnostics.batch_receive_aggregate_seconds={:.6}",
        seconds(&BATCH_RECEIVE_NS)
    );
    println!(
        "diagnostics.transaction_start_aggregate_seconds={:.6}",
        seconds(&TRANSACTION_START_NS)
    );
    println!(
        "diagnostics.star_copy_aggregate_seconds={:.6}",
        seconds(&STAR_COPY_NS)
    );
    println!(
        "diagnostics.planet_copy_aggregate_seconds={:.6}",
        seconds(&PLANET_COPY_NS)
    );
    println!(
        "diagnostics.commit_aggregate_seconds={:.6}",
        seconds(&COMMIT_NS)
    );
    println!("diagnostics.transactions={}", batches);
    println!("diagnostics.batch_size_min={}", min_batch_size);
    println!("diagnostics.batch_size_average={:.3}", average_batch_size);
    println!(
        "diagnostics.batch_size_max={}",
        MAX_BATCH_SIZE.load(Relaxed)
    );
}
