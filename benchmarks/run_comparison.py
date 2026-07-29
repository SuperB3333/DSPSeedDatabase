#!/usr/bin/env python3
"""Compare two inserter executables with controlled process runs."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import random
import re
import statistics
import subprocess
import tempfile
from datetime import datetime, timezone
from pathlib import Path


THROUGHPUT_PATTERN = re.compile(r"^seeds/sec: ([0-9.]+)$", re.MULTILINE)
TIME_FIELDS = (
    "wall_seconds",
    "user_seconds",
    "system_seconds",
    "max_rss_kib",
    "minor_page_faults",
    "major_page_faults",
    "voluntary_context_switches",
    "involuntary_context_switches",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--candidate-name", required=True)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--start-seed", type=int, default=0)
    parser.add_argument("--end-seed", type=int, default=60)
    parser.add_argument("--workers", type=int, default=1)
    parser.add_argument("--rounds", type=int, default=3)
    parser.add_argument("--cpus", default="0")
    return parser.parse_args()


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    position = (len(ordered) - 1) * fraction
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    weight = position - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def describe(values: list[float]) -> dict[str, float]:
    median = statistics.median(values)
    deviations = [abs(value - median) for value in values]
    return {
        "median": median,
        "mad": statistics.median(deviations),
        "p25": percentile(values, 0.25),
        "p75": percentile(values, 0.75),
        "minimum": min(values),
        "maximum": max(values),
    }


def bootstrap_ci(values: list[float], samples: int = 20_000) -> list[float]:
    generator = random.Random(0)
    medians = []
    for _ in range(samples):
        draw = [generator.choice(values) for _ in values]
        medians.append(statistics.median(draw))
    return [percentile(medians, 0.025), percentile(medians, 0.975)]


def run_once(
    label: str,
    executable: Path,
    args: argparse.Namespace,
    sequence: int,
) -> dict[str, object]:
    environment = os.environ.copy()
    environment.update(
        {
            "BENCHMARK": "1",
            "START_SEED": str(args.start_seed),
            "END_SEED": str(args.end_seed),
            "WORKER_THREADS": str(args.workers),
            "WRITER_THREADS": "1",
            "CHANNEL_SIZE": "64",
            "COMMIT_COUNT": "64",
            "NO_TUI": "1",
            "LOG_LEVEL": "error",
            "LOG_INTERVAL": "none",
            "CHECKPOINT_INTERVAL": "none",
            "OVERRIDE_CHECKPOINTS": "1",
            "CHECKPOINT_FILE": "/tmp/dsp-benchmark-no-checkpoint",
            "PG_NETLOC": "127.0.0.1",
            "PG_PORT": "1",
        }
    )

    with tempfile.NamedTemporaryFile() as time_output:
        command = [
            "/usr/bin/time",
            "-f",
            "%e\n%U\n%S\n%M\n%R\n%F\n%w\n%c",
            "-o",
            time_output.name,
            "taskset",
            "-c",
            args.cpus,
            str(executable),
        ]
        completed = subprocess.run(
            command,
            env=environment,
            check=False,
            text=True,
            capture_output=True,
        )
        time_output.seek(0)
        measurements = time_output.read().decode("ascii").splitlines()

    if completed.returncode != 0:
        raise RuntimeError(
            f"{label} failed with status {completed.returncode}:\n{completed.stderr}"
        )
    match = THROUGHPUT_PATTERN.search(completed.stdout)
    if match is None or len(measurements) != len(TIME_FIELDS):
        raise RuntimeError(f"Cannot parse {label} output:\n{completed.stdout}")

    values = dict(zip(TIME_FIELDS, measurements, strict=True))
    integer_fields = set(TIME_FIELDS[3:])
    result: dict[str, object] = {
        "sequence": sequence,
        "variant": label,
        "throughput_seeds_per_second": float(match.group(1)),
    }
    for field, value in values.items():
        result[field] = int(value) if field in integer_fields else float(value)
    return result


def summarize(results: list[dict[str, object]], candidate_name: str) -> dict[str, object]:
    by_variant = {
        label: [
            float(result["throughput_seeds_per_second"])
            for result in results
            if result["variant"] == label
        ]
        for label in ("baseline", candidate_name)
    }
    baseline = by_variant["baseline"]
    candidate = by_variant[candidate_name]
    paired_changes = [
        (new / old - 1.0) * 100.0 for old, new in zip(baseline, candidate, strict=True)
    ]
    return {
        "throughput": {
            "baseline": describe(baseline),
            candidate_name: describe(candidate),
        },
        "paired_change_percent": {
            **describe(paired_changes),
            "bootstrap_95_percent_ci": bootstrap_ci(paired_changes),
        },
    }


def main() -> None:
    args = parse_args()
    for executable in (args.baseline, args.candidate):
        if not executable.is_file():
            raise SystemExit(f"Executable does not exist: {executable}")

    variants = {
        "baseline": args.baseline.resolve(),
        args.candidate_name: args.candidate.resolve(),
    }
    order = ["baseline", args.candidate_name, args.candidate_name, "baseline"]
    results = []
    for _ in range(args.rounds):
        for label in order:
            result = run_once(label, variants[label], args, len(results))
            results.append(result)
            print(
                f"{label}: {result['throughput_seeds_per_second']:.6f} seeds/s",
                flush=True,
            )

    artifact = {
        "schema_version": 1,
        "created_utc": datetime.now(timezone.utc).isoformat(),
        "host": {
            "platform": platform.platform(),
            "processor_count": os.cpu_count(),
            "cpu_set": args.cpus,
        },
        "workload": {
            "start_seed": args.start_seed,
            "end_seed": args.end_seed,
            "seed_count": args.end_seed - args.start_seed,
            "workers": args.workers,
            "writer_sinks": 1,
            "channel_size": 64,
        },
        "executables": {
            label: {"path": str(path), "sha256": file_sha256(path)}
            for label, path in variants.items()
        },
        "results": results,
        "summary": summarize(results, args.candidate_name),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(artifact, indent=2) + "\n", encoding="ascii")
    print(json.dumps(artifact["summary"], indent=2))


if __name__ == "__main__":
    main()
