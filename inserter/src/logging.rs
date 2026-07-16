//! Lightweight, dependency-free console logging.
//!
//! Verbosity is controlled by the `LOG_LEVEL` env var (default `info`):
//!   error < warn < info < debug
//!
//! Logging is intentionally cheap: a level check is a single atomic load and
//! per-message formatting only happens when the level is enabled. Nothing here
//! runs in the per-seed hot loop, so throughput is unaffected.

use std::sync::LazyLock;

pub const LEVEL_ERROR: u8 = 0;
pub const LEVEL_WARN: u8 = 1;
pub const LEVEL_INFO: u8 = 2;
pub const LEVEL_DEBUG: u8 = 3;

static LOG_LEVEL: LazyLock<u8> = LazyLock::new(|| {
    match std::env::var("LOG_LEVEL")
        .unwrap_or_else(|_| "info".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "error" => LEVEL_ERROR,
        "warn" | "warning" => LEVEL_WARN,
        "info" => LEVEL_INFO,
        "debug" | "trace" => LEVEL_DEBUG,
        other => {
            eprintln!("[warn] unknown LOG_LEVEL '{}', defaulting to 'info'", other);
            LEVEL_INFO
        }
    }
});

#[inline]
pub fn enabled(level: u8) -> bool {
    level <= *LOG_LEVEL
}

/// Internal: write a tagged line to stderr so it never mixes with data on stdout.
#[inline]
pub fn log_line(tag: &str, msg: &str, col: &str) { eprintln!("\x1b[{}m[{}] {}\x1b[0m", col, tag, msg); }

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        if $crate::logging::enabled($crate::logging::LEVEL_ERROR) {
            $crate::logging::log_line("error", &format!($($arg)*), "9");
        }
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        if $crate::logging::enabled($crate::logging::LEVEL_WARN) {
            $crate::logging::log_line("warn", &format!($($arg)*), "11");
        }
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        if $crate::logging::enabled($crate::logging::LEVEL_INFO) {
            $crate::logging::log_line("info", &format!($($arg)*), "2");
        }
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        if $crate::logging::enabled($crate::logging::LEVEL_DEBUG) {
            $crate::logging::log_line("debug", &format!($($arg)*), "8");
        }
    };
}
#[macro_export]
macro_rules! error_return {
    ($($arg:tt)*) => {
        $crate::log_error!($($arg)*);
        return Err(anyhow::anyhow!($($arg)*));
    }
}
