use std::io::Write;
use std::ops::Range;
use std::time::{Duration, Instant};
use std::path::Path;
use crate::{END_SEED, MAX_WORKERS, START_SEED, WORKER_THREADS};

pub const COPY_PLANET: &str = "COPY planets(star_id, index, orbiting, water_item, gas_giant, sun_distance, inside_ds, satellites, temperature, theme_id, gas_h, gas_d, gas_i, tidal_lock, min_iron, max_iron, estimate_iron, min_copper, max_copper, estimate_copper, min_silicium, max_silicium, estimate_silicium, min_titanium, max_titanium, estimate_titanium, min_stone, max_stone, estimate_stone, min_coal, max_coal, estimate_coal, min_oil, max_oil, estimate_oil, min_fireice, max_fireice, estimate_fireice, min_diamond, max_diamond, estimate_diamond, min_fractal, max_fractal, estimate_fractal, min_crysrub, max_crysrub, estimate_crysrub, min_grat, max_grat, estimate_grat, min_bamboo, max_bamboo, estimate_bamboo, min_mag, max_mag, estimate_mag) FROM STDIN WITH (FORMAT CSV)";
pub const COPY_STAR: &str = "COPY stars(id, seed, start_dist, star_index, luminosity, dyson_radius, type, spectr, ore_iron, ore_copper, ore_silicium, ore_titanium, ore_stone, ore_coal, ore_oil, ore_fireice, ore_diamond, ore_fractal, ore_crysrub, ore_grat, ore_bamboo, ore_mag) FROM STDIN WITH (FORMAT CSV)";

const INIT_SCRIPT: &str = include_str!("init.sql");
pub const TRUTHY: [&str; 5] = ["1", "true", "yes", "enable", "on"];
pub fn split_chunks(r: Range<i32>, chunks: i32) -> Vec<Range<i32>> {
    let total = r.end - r.start;
    let base = total / chunks;
    let mut extra = total % chunks;
    let mut cur = r.start;
    let mut out = Vec::with_capacity(chunks as usize);
    for _ in 0..chunks {
        let add = if extra > 0 {
            extra -= 1;
            base + 1
        } else {
            base
        };
        out.push(cur..(cur + add));
        cur += add;
    }
    out
}
#[macro_export]
macro_rules! env_int {
    ($var:expr) => {
        match std::env::var($var) {
            Ok(val) => val.parse(),
            Err(err) => Err(err),
        }
    };
    ($var:expr, $default:expr) => {
        std::env::var($var)
            .map(|e| e.parse::<i32>().unwrap_or($default))
            .unwrap_or($default)
    };
}
#[macro_export]
macro_rules! env_bool {
    ($var:expr) => {
        env_bool!($var, false)
    };
    ($var:expr, $default:expr) => {
        match std::env::var($var) {
            Ok(value) => crate::misc::TRUTHY.contains(&value.to_ascii_lowercase().as_str()),
            Err(_) => false
        }
    }
}
#[macro_export]
macro_rules! env_str {
    ($var:expr) => {
        std::env::var($var)
    };
    ($var:expr, $default:expr) => {
        std::env::var($var).unwrap_or($default.to_string())
    };
}
#[inline]
pub fn get_env_interval(name: &str) -> Option<Duration> {
    let envvar = env_str!(name, "medium");
    // if it is a number, interpret it as missiseconds
    if let Ok(num) = envvar.parse::<u64>() {
        return Some(Duration::from_millis(num));
    }

    match crate::process_option!(envvar).as_str() {
        "none" => None,
        "verylow" => Some(Duration::from_secs(60)),
        "low" => Some(Duration::from_secs(30)),
        "medium" => Some(Duration::from_secs(10)),
        "high" => Some(Duration::from_secs(1)),
        "veryhigh" | "xhigh" => Some(Duration::from_millis(250)),
        "realtime" | "atomic" => Some(Duration::from_millis(100)),
        val => {
            crate::log_warn!("Invalid value for CHECKPOINT_INTERVAL! Using 10 sec as default. Value: {}", val);
            Some(Duration::from_secs(10))
        }
    }

}
/// Creates the tables in the database, as well as the user for the api. Returns whether the database was set up before
pub fn create_db_schema() -> bool {
    postgres::Client::connect((*crate::DB_STR).as_str(), postgres::NoTls)
        .unwrap()
        .batch_execute(INIT_SCRIPT)
        .is_err()
}

pub fn check_db_connection() -> bool {
    crate::log_info!(
        "Checking database connection to {}",
        *crate::DB_STR
    );

    match postgres::Client::connect((*crate::DB_STR).as_str(), postgres::NoTls) {
        Ok(mut client) => match client.simple_query("SELECT 1") {
            Ok(_) => {
                crate::log_info!("Database connection check succeeded");
                true
            }
            Err(err) => {
                crate::log_error!("Database connection check query failed: {}", err);
                false
            }
        },
        Err(err) => {
            crate::log_error!("Database connection failed: {}", err);
            false
        }
    }
}

pub struct Timer {
    pub interval: Duration,
    last: Instant,
}
impl Timer {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last: Instant::now(),
        }
    }
    pub fn reset(&mut self) {
        self.last = Instant::now();
    }
    pub fn is_ready(&self) -> bool {
        Instant::now().duration_since(self.last) >= self.interval
    }
    pub fn is_ready_autoreset(&mut self) -> bool {
        if !self.is_ready() { return false }
        self.reset();
        true

    }
}
pub fn print_effective_config() {
    println!("config.start_seed={}", *crate::START_SEED);
    println!("config.end_seed={}", *crate::END_SEED);
    println!("config.seed_count={}", *crate::END_SEED - *crate::START_SEED);
    println!("config.worker_threads={}", *crate::WORKER_THREADS);
    println!("config.writer_threads={}", *crate::WRITER_THREADS);
    println!("config.commit_count={}", *crate::COMMIT_COUNT);
    println!("config.channel_size={}", *crate::CHANNEL_SIZE);
    println!("config.benchmark={}", *crate::BENCHMARK);
    println!("config.checkpoint_file={}", *crate::CHECKPOINT_FILE);
    println!("config.database.user={}", *crate::PG_USER);
    println!("config.database.host={}", *crate::PG_NETLOC);
    println!("config.database.port={}", *crate::PG_PORT);
    println!("config.database.name={}", *crate::PG_DBNAME);
    println!("config.database.password=redacted");
    println!("config.log_level={}", std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()));
    println!("config.print_config=true");
}
pub fn validate_config() -> anyhow::Result<()> {
    if *crate::WORKER_THREADS <= 0 {
        crate::error_return!("WORKER_THREADS must be greater than 0");
    }
    if *crate::WRITER_THREADS <= 0 {
        crate::error_return!("WRITER_THREADS must be greater than 0");
    }
    if *crate::COMMIT_COUNT == 0 {
        crate::error_return!("COMMIT_COUNT must be greater than 0");
    }

    if *START_SEED > *END_SEED {
        crate::error_return!("START_SEED is higher than END_SEED");
    }
    if *WORKER_THREADS > (*END_SEED - *START_SEED) {
        crate::error_return!("More worker threads than seeds to process");
    }
    if *WORKER_THREADS > MAX_WORKERS as i32 {
        crate::error_return!("More worker threads than the binary allows! Try to compile with MAX_WORKERS set higher.");
    }
    
    if *crate::TUI_MODE == crate::metrics::TUIMode::Diagnosis {
        crate::log_warn!("Diagnosis TUI Mode is not yet implemented!");
    }
    Ok(())
}

pub fn write_log() -> anyhow::Result<()> {
    let log_path = Path::new(&*crate::LOG_DIR).join(&*crate::LOG_NAME);

    if !log_path.exists() {
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&log_path)?
            .write_all(b"CSV\tHEADER")?; //todo correct csv header
    }
    // Write entry to perf log
    std::fs::OpenOptions::new()
        .append(true)
        .open(&log_path)?
        .write_fmt(format_args!("\nTODO: IMPLEMENT LOG :-)"))?; //todo select data to log

    Ok(())
}

#[macro_export]
macro_rules! process_option {
    ($user_option:expr) => {
        $user_option.to_ascii_lowercase().replace(" ", "").replace("_", "").replace("-", "")
    };
}