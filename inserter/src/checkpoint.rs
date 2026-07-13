use crate::misc::{create_db_schema, split_chunks};
use crate::{
    log_info, log_warn, BENCHMARK, CHECKPOINT_FILE, CHECKPOINT_OVERWRITE, END_SEED,
    PROGRESS_WORKERS, START_SEED, WORKER_THREADS,
};
use anyhow::{anyhow, Context, Result};
use std::fs::{self, File};
use std::io::Write;
use std::ops::Range;
use std::sync::atomic::Ordering::Relaxed;
use std::time::Duration;

const CHECKPOINT_REWIND_SEEDS: i32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointFrequency {
    None,
    VeryLow,
    Low,
    Medium,
    High,
    XHigh,
    Atomic,
}

impl CheckpointFrequency {
    pub fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "none" => Ok(Self::None),
            "very_low" => Ok(Self::VeryLow),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "xhigh" => Ok(Self::XHigh),
            "atomic" => Ok(Self::Atomic),
            _ => Err(anyhow!(
                "invalid CHECKPOINT_FREQUENCY; expected none, very_low, low, medium, high, xhigh, or atomic"
            )),
        }
    }

    pub fn interval(self) -> Option<Duration> {
        match self {
            Self::None => None,
            Self::VeryLow => Some(Duration::from_secs(60)),
            Self::Low => Some(Duration::from_secs(30)),
            Self::Medium => Some(Duration::from_secs(10)),
            Self::High => Some(Duration::from_secs(1)),
            Self::XHigh => Some(Duration::from_millis(250)),
            Self::Atomic => Some(Duration::from_millis(100)),
        }
    }
}

pub fn write_checkpoints() -> Result<()> {
    let path = std::path::Path::new(CHECKPOINT_FILE.as_str());
    let workload_starts: Vec<i32> = split_chunks(*START_SEED..*END_SEED, *WORKER_THREADS)
        .into_iter()
        .map(|workload| workload.start)
        .collect();
    let progress: Vec<i32> = PROGRESS_WORKERS
        .iter()
        .take(*WORKER_THREADS as usize)
        .map(|progress| progress.load(Relaxed))
        .collect();
    let values = checkpoint_values(&workload_starts, &progress);
    write_checkpoint_values(path, &values)
}

fn checkpoint_values(workload_starts: &[i32], progress: &[i32]) -> Vec<i32> {
    workload_starts
        .iter()
        .zip(progress)
        .map(|(start, progress)| (*start).max(*start + *progress - CHECKPOINT_REWIND_SEEDS))
        .collect()
}

fn write_checkpoint_values(path: &std::path::Path, values: &[i32]) -> Result<()> {
    let temporary_path = path.with_extension("tmp");
    let mut file = File::create(&temporary_path).with_context(|| {
        format!(
            "failed to create temporary checkpoint file {}",
            temporary_path.display()
        )
    })?;

    for value in values {
        writeln!(file, "{}", value)?;
    }
    file.sync_all()?;
    fs::rename(&temporary_path, path).with_context(|| {
        format!(
            "failed to replace checkpoint file {} with {}",
            path.display(),
            temporary_path.display()
        )
    })?;
    Ok(())
}

/// Loads potential checkpoints or defaults to full seeds. Also creates db schema and looks for potential corrupted data
pub fn load_workloads() -> Result<Vec<Range<i32>>> {
    let normal_workloads = split_chunks(*START_SEED..*END_SEED, *WORKER_THREADS);

    let (workloads, had_checkpoints) = match read_checkpoints(&normal_workloads)? {
        Some(workloads) => {
            log_info!("Read checkpoints successfully");
            (workloads, true)
        }
        None => {
            log_info!("No checkpoints found; distributing all seeds among generators");
            (normal_workloads, false)
        }
    };

    if !*BENCHMARK && !(create_db_schema() == had_checkpoints) {
        log_warn!(
            "Found checkpoints and no db schema or the other way around. Data might be corrupt!"
        )
    }

    Ok(workloads)
}
fn read_checkpoints(normal_workloads: &[Range<i32>]) -> Result<Option<Vec<Range<i32>>>> {
    let path = std::path::Path::new(CHECKPOINT_FILE.as_str());
    if *CHECKPOINT_OVERWRITE {
        log_info!("CHECKPOINT_OVERWRITE=1; ignoring existing checkpoints");
        if path.exists() {
            fs::remove_file(path)
                .with_context(|| format!("failed to remove checkpoint file {}", path.display()))?;
        }
        return Ok(None);
    }

    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read checkpoint file {}", path.display()))?;
    let starts: Vec<i32> = contents
        .lines()
        .map(|line| line.parse::<i32>().context("invalid checkpoint value"))
        .collect::<Result<_>>()?;

    if starts.len() != normal_workloads.len() {
        return Err(anyhow!(
            "checkpoint file has {} entries but WORKER_THREADS={} requires {} entries; set CHECKPOINT_OVERWRITE=1 to ignore it",
            starts.len(),
            *WORKER_THREADS,
            normal_workloads.len()
        ));
    }

    workloads_from_starts(starts, normal_workloads).map(Some)
}

fn workloads_from_starts(starts: Vec<i32>, normal_workloads: &[Range<i32>]) -> Result<Vec<Range<i32>>> {
    starts
        .into_iter()
        .zip(normal_workloads)
        .map(|(start, workload)| {
            if !(start >= workload.start && start <= workload.end) {
                return Err(anyhow!(
                    "checkpoint start {} is outside {}..={}; set CHECKPOINT_OVERWRITE=1 to ignore it",
                    start,
                    workload.start,
                    workload.end
                ));
            }
            Ok(start..workload.end)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        checkpoint_values, workloads_from_starts, write_checkpoint_values, CheckpointFrequency,
    };
    use std::fs;
    use std::time::Duration;

    #[test]
    fn parses_each_checkpoint_frequency() {
        assert_eq!(CheckpointFrequency::parse("none").unwrap().interval(), None);
        assert_eq!(
            CheckpointFrequency::parse("very_low").unwrap().interval(),
            Some(Duration::from_secs(60))
        );
        assert_eq!(
            CheckpointFrequency::parse("low").unwrap().interval(),
            Some(Duration::from_secs(30))
        );
        assert_eq!(
            CheckpointFrequency::parse("medium").unwrap().interval(),
            Some(Duration::from_secs(10))
        );
        assert_eq!(
            CheckpointFrequency::parse("high").unwrap().interval(),
            Some(Duration::from_secs(1))
        );
        assert_eq!(
            CheckpointFrequency::parse("xhigh").unwrap().interval(),
            Some(Duration::from_millis(250))
        );
        assert_eq!(
            CheckpointFrequency::parse("atomic").unwrap().interval(),
            Some(Duration::from_millis(100))
        );
    }

    #[test]
    fn rejects_unknown_checkpoint_frequency() {
        assert!(CheckpointFrequency::parse("fast").is_err());
    }

    #[test]
    fn checkpoint_write_replaces_existing_contents() {
        let directory =
            std::env::temp_dir().join(format!("dsp-checkpoint-test-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let checkpoint = directory.join("checkpoints.txt");
        fs::write(&checkpoint, "stale\n").unwrap();

        write_checkpoint_values(&checkpoint, &[10, 20]).unwrap();

        assert_eq!(fs::read_to_string(&checkpoint).unwrap(), "10\n20\n");
        assert!(!checkpoint.with_extension("tmp").exists());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn checkpoint_values_use_absolute_seeds_and_never_precede_a_workload() {
        assert_eq!(
            checkpoint_values(&[0, 1_000, 2_000], &[0, 1, 100]),
            vec![0, 1_000, 2_098]
        );
    }

    #[test]
    fn checkpoint_starts_must_stay_within_their_worker_workload() {
        let workloads = [0..10, 10..20];
        assert!(workloads_from_starts(vec![0, 9], &workloads).is_err());
        assert_eq!(
            workloads_from_starts(vec![5, 15], &workloads).unwrap(),
            vec![5..10, 15..20]
        );
    }
}
