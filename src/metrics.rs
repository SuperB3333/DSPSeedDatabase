use std::io::{stdout, Write};
use crossterm::{
    ExecutableCommand,
    terminal::{Clear, ClearType},
};
use crossterm::cursor::MoveTo;
use super::COMMITTED_SEEDS;
use crate::log_info;

pub fn write_metrics(sps: f32, goal: i32, queue: i32) -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = stdout();

    stdout.execute(MoveTo(0, 0))?;
    stdout.execute(Clear(ClearType::All))?;

    writeln!(stdout, "Live metrics:")?;
    writeln!(stdout, "seeds/sec: {:<5}", format!("{:.2}", sps))?;

    let cpu_progress = (COMMITTED_SEEDS.load(std::sync::atomic::Ordering::SeqCst) + queue) as f32 / goal as f32;
    // Clamp to 100 so "░".repeat(100 - p) never underflows when committed+queue > goal.
    let cpu_percent = ((cpu_progress * 100.0).round() as usize).min(100);
    writeln!(stdout, "Calculation Progress {:<3}%: [{}{}]", cpu_percent, "█".repeat(cpu_percent), "░".repeat(100 - cpu_percent))?;

    let db_progress = COMMITTED_SEEDS.load(std::sync::atomic::Ordering::SeqCst) as f32 / goal as f32;
    // Clamp to 100 so "░".repeat(100 - p) never underflows.
    let db_percent = ((db_progress * 100.0).round() as usize).min(100);
    writeln!(stdout, "Writing Progress {:<3}%: [{}{}]", db_percent, "█".repeat(db_percent), "░".repeat(100 - db_percent))?;

    Ok(())
}

/// stderr progress fallback for when the TUI is disabled (non-TTY or NO_TUI=1).
/// Emits exactly ONE plain line via `log_info!` so it lands on stderr only.
pub fn log_progress(sps: f32, goal: i32, queue: i32) {
    let committed = COMMITTED_SEEDS.load(std::sync::atomic::Ordering::SeqCst);
    let percent = if goal > 0 {
        ((committed as f32 / goal as f32) * 100.0).round() as i32
    } else {
        0
    };
    log_info!(
        "progress: committed={}/{} ({}%), in-flight={}, seeds/sec={:.2}",
        committed, goal, percent, queue, sps
    );
}