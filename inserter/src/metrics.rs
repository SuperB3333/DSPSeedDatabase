use std::io::{stdout, Write};
use crossterm::{
    ExecutableCommand,
    terminal::{Clear, ClearType, enable_raw_mode, disable_raw_mode},
    cursor::MoveTo,
    tty::IsTty,
};
use super::{env_bool, COMMITTED_SEEDS};

pub fn write_metrics(sps: f32, goal: i32, queue: i32) -> anyhow::Result<()> {
    let mode = &*crate::TUI_MODE;
    
    enable_raw_mode()?;
    let mut stdout = stdout();

    stdout.execute(MoveTo(0, 0))?;
    for _ in 0..4 { stdout.execute(Clear(ClearType::All))?; }
    
    if mode >= &TUIMode::SeedsSec {
        writeln!(stdout, "Live metrics:")?;
        writeln!(stdout, "seeds/sec: {:<5}", format!("{:.2}", sps))?;
    }
    
    if mode >= &TUIMode::Progress {
        let cpu_progress = (COMMITTED_SEEDS.load(std::sync::atomic::Ordering::SeqCst) + queue) as f32 / goal as f32;
        let cpu_percent = (cpu_progress * 100.0).round() as usize;
        writeln!(stdout, "Calculation Progress {:<3}%: [{}{}]", cpu_percent, "█".repeat(cpu_percent), "░".repeat(100 - cpu_percent))?;
        
        let db_progress = COMMITTED_SEEDS.load(std::sync::atomic::Ordering::SeqCst) as f32 / goal as f32;
        let db_percent = (db_progress * 100.0).round() as usize;
        writeln!(stdout, "Writing Progress {:<3}%: [{}{}]", db_percent, "█".repeat(db_percent), "░".repeat(100 - db_percent))?;
    }
    if mode >= &TUIMode::QueueFill {
        let status = queue_status(queue);
        writeln!(stdout, "Queue status: {} ({:.2}%)", status.1, status.0)?;
    }
    
    // bottleneck disgnosis

    disable_raw_mode()?;
    Ok(())
}
#[cfg(windows)]
#[inline]
fn supports_ansi() -> bool { crossterm::ansi_support::supports_ansi() }
#[cfg(not(windows))]
#[inline]
fn supports_ansi() -> bool { true }

#[inline]
fn queue_status(queue: i32) -> (f32, &'static str) {
    let part = queue as f32 / *crate::CHANNEL_SIZE as f32;

    (part * 100.0, match part {
        0.0..0.2 => "Empty",
        0.2..0.8=> "Balanced",
        0.8..1.0 => "Full",
        _ => panic!("Channel larger than max size")
    })
    
}
pub fn get_tui_mode() -> TUIMode {
    if !(supports_ansi() && stdout().is_tty() && !env_bool!("NO_TUI")) {
        return TUIMode::Off;
    }
    let var = crate::env_str!("TUI_MODE", "2");

    if let Ok(num) = var.parse::<u8>() {
        return TUIMode::from(num);
    }

    let cleaned = crate::process_option!(var);

    if let Ok(mode) = TUIMode::try_from(cleaned.as_str()) { return mode; }

    if crate::misc::TRUTHY.contains(&cleaned.as_str()) { return TUIMode::SeedsSec; }

    TUIMode::Off


}
#[derive(PartialOrd, PartialEq)]
#[repr(u8)]
pub enum TUIMode {
    Off = 0,
    Progress = 1,
    SeedsSec = 2,
    QueueFill = 3,
    Diagnosis = 4,
}

impl From<TUIMode> for u8 {
    fn from(mode: TUIMode) -> u8 {
        mode as u8
    }
}

impl From<u8> for TUIMode {
    fn from(value: u8) -> Self {
        match value {
            0 => TUIMode::Off,
            1 => TUIMode::Progress,
            2 => TUIMode::SeedsSec,
            3 => TUIMode::QueueFill,
            4 => TUIMode::Diagnosis,
            _ => TUIMode::SeedsSec,
        }
    }
}
impl TryFrom<&str> for TUIMode {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "off" | "notui" => Ok(TUIMode::Off),
            "progress" => Ok(TUIMode::Progress),
            "seedssed" | "speed" => Ok(TUIMode::SeedsSec),
            "queuefill" => Ok(TUIMode::QueueFill),
            "diagnose" | "analyze" | "full" => Ok(TUIMode::Diagnosis),
            _ => Err(()),
        }
    }
}