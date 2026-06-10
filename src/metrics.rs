use std::io::{stdout, Write};
use crossterm::{
    ExecutableCommand,
    terminal::{Clear, ClearType, enable_raw_mode, disable_raw_mode},
};
use crossterm::cursor::MoveTo;
use super::COMMITTED_SEEDS;

pub fn write_metrics(sps: f32, goal: i32, queue: i32) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = stdout();

    stdout.execute(MoveTo(0, 0))?;
    for _ in 0..4 { stdout.execute(Clear(ClearType::All))?; }

    writeln!(stdout, "Live metrics:")?;
    writeln!(stdout, "seeds/sec: {:<5}", format!("{:.2}", sps))?;

    let cpu_progress = (COMMITTED_SEEDS.load(std::sync::atomic::Ordering::SeqCst) + queue) as f32 / goal as f32;
    let cpu_percent = (cpu_progress * 100.0).round() as usize;
    writeln!(stdout, "Calculation Progress {:<3}%: [{}{}]", cpu_percent, "█".repeat(cpu_percent), "░".repeat(100 - cpu_percent))?;

    let db_progress = COMMITTED_SEEDS.load(std::sync::atomic::Ordering::SeqCst) as f32 / goal as f32;
    let db_percent = (db_progress * 100.0).round() as usize;
    writeln!(stdout, "Writing Progress {:<3}%: [{}{}]", db_percent, "█".repeat(db_percent), "░".repeat(100 - db_percent))?;


    disable_raw_mode()?;
    Ok(())
}