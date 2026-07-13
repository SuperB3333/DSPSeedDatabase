#!/usr/bin/env python3
"""Measure DSP seed pipeline scaling and recommend environment settings."""

from __future__ import annotations

import argparse
import os
import random
import shlex
import statistics
import subprocess
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Iterable


@dataclass(frozen=True)
class Sweep:
    name: str
    env_name: str
    candidates: tuple[int, ...]
    database_writes: bool


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("must be greater than 0")
    return parsed


def candidate_values(value: str) -> tuple[int, ...]:
    try:
        parsed = tuple(dict.fromkeys(int(item.strip()) for item in value.split(",")))
    except ValueError as error:
        raise argparse.ArgumentTypeError("must be comma-separated integers") from error
    if not parsed or any(item <= 0 for item in parsed):
        raise argparse.ArgumentTypeError("all candidates must be greater than 0")
    return parsed


def efficiency_percent(value: str) -> float:
    parsed = float(value)
    if not 0.0 < parsed <= 100.0:
        raise argparse.ArgumentTypeError("must be greater than 0 and at most 100")
    return parsed


def build_command(template: str, environment: dict[str, str]) -> list[str]:
    command: list[str] = []
    for token in shlex.split(template):
        if token == "{docker_env}":
            for name in sorted(environment):
                command.extend(("-e", f"{name}={environment[name]}"))
        elif "{docker_env}" in token:
            raise ValueError("{docker_env} must be a separate command token")
        else:
            command.append(token)
    if not command:
        raise ValueError("command cannot be empty")
    return command


def recommend(results: dict[int, list[float]], threshold: float) -> tuple[int, int]:
    medians = {
        candidate: statistics.median(samples)
        for candidate, samples in results.items()
        if samples
    }
    if not medians:
        raise ValueError("no successful measurements")
    peak_candidate = max(medians, key=lambda candidate: medians[candidate])
    minimum_acceptable = medians[peak_candidate] * threshold
    recommended = min(
        candidate
        for candidate, median in medians.items()
        if median >= minimum_acceptable
    )
    return recommended, peak_candidate


def update_env_file(path: Path, updates: dict[str, int]) -> None:
    lines = path.read_text().splitlines() if path.exists() else []
    remaining = {name: str(value) for name, value in updates.items()}
    output: list[str] = []

    for line in lines:
        stripped = line.strip()
        name = stripped.split("=", 1)[0] if "=" in stripped else ""
        if name in remaining and not stripped.startswith("#"):
            output.append(f"{name}={remaining.pop(name)}")
        else:
            output.append(line)

    if remaining and output and output[-1] != "":
        output.append("")
    output.extend(f"{name}={value}" for name, value in remaining.items())

    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        "w", dir=path.parent, prefix=f".{path.name}.", delete=False
    ) as temporary:
        temporary.write("\n".join(output) + "\n")
        temporary_path = Path(temporary.name)
    temporary_path.replace(path)


def confirmed_updates(
    updates: dict[str, int],
    path: Path,
    read_answer: Callable[[str], str] = input,
) -> dict[str, int]:
    accepted: dict[str, int] = {}
    for name, value in updates.items():
        answer = read_answer(f"Write {name}={value} to {path}? [y/N] ")
        if answer.strip().lower() in {"y", "yes"}:
            accepted[name] = value
        else:
            print(f"Skipped {name}; continuing with remaining recommendations.")
    return accepted


def measure(
    command_template: str,
    overrides: dict[str, str],
    seeds: int,
) -> tuple[float | None, str]:
    command = build_command(command_template, overrides)
    environment = os.environ.copy()
    environment.update(overrides)
    started = time.perf_counter()
    completed = subprocess.run(
        command,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    elapsed = time.perf_counter() - started
    output = completed.stdout + completed.stderr
    if completed.returncode != 0:
        return None, output
    return seeds / elapsed, output


def print_results(
    sweep: Sweep,
    results: dict[int, list[float]],
    threshold: float,
) -> int | None:
    successful = {candidate: samples for candidate, samples in results.items() if samples}
    print(f"\n{sweep.name} results:")
    if not successful:
        print("  no successful trials")
        return None

    medians = {
        candidate: statistics.median(samples)
        for candidate, samples in successful.items()
    }
    peak_throughput = max(medians.values())
    for candidate in sweep.candidates:
        samples = successful.get(candidate)
        if not samples:
            print(f"  {sweep.env_name}={candidate}: failed")
            continue
        median = medians[candidate]
        print(
            f"  {sweep.env_name}={candidate}: "
            f"median={median:.6f} seeds/sec "
            f"min={min(samples):.6f} max={max(samples):.6f} "
            f"peak={median / peak_throughput * 100.0:.2f}% samples={len(samples)}"
        )

    recommended, peak_candidate = recommend(successful, threshold)
    efficient_ceiling = max(
        candidate
        for candidate, median in medians.items()
        if median >= peak_throughput * threshold
    )
    print(
        f"  recommendation: {sweep.env_name}={recommended} "
        f"(smallest candidate at or above {threshold * 100.0:.2f}% of peak; "
        f"peak candidate={peak_candidate}; efficient ceiling={efficient_ceiling})"
    )
    return recommended


def run_sweep(
    sweep: Sweep,
    args: argparse.Namespace,
    seed_cursor: int,
    checkpoint_directory: Path,
) -> tuple[int | None, int]:
    results = {candidate: [] for candidate in sweep.candidates}
    randomizer = random.Random(args.order_seed)

    for repetition in range(1, args.repetitions + 1):
        candidates = list(sweep.candidates)
        randomizer.shuffle(candidates)
        for candidate in candidates:
            start_seed = seed_cursor
            end_seed = start_seed + args.seeds_per_trial
            seed_cursor = end_seed
            checkpoint_name = f"dsp-tuning-{sweep.name}-{candidate}-{repetition}.txt"
            checkpoint = (
                Path("/tmp") / checkpoint_name
                if "{docker_env}" in args.command
                else checkpoint_directory / checkpoint_name
            )
            overrides = {
                "START_SEED": str(start_seed),
                "END_SEED": str(end_seed),
                "WORKER_THREADS": str(args.base_workers),
                "WRITER_THREADS": str(args.base_writers),
                "COMMIT_COUNT": str(args.base_commit_count),
                "CHANNEL_SIZE": str(args.channel_size),
                "CHECKPOINT_FILE": "/dev/null" if args.test_only else str(checkpoint),
                "NO_TUI": "1",
                "LOG_LEVEL": "error",
                "BENCHMARK": "1" if sweep.name == "worker" else "0",
                "TEST_ONLY": "1" if args.test_only else "0",
                sweep.env_name: str(candidate),
            }
            command = build_command(args.command, overrides)
            print(
                f"[{sweep.name} {repetition}/{args.repetitions}] "
                f"{sweep.env_name}={candidate} seeds={start_seed}..{end_seed}"
            )
            if args.dry_run:
                print("  command:", shlex.join(command))
                continue

            throughput, output = measure(
                args.command, overrides, args.seeds_per_trial
            )
            if throughput is None:
                print("  failed; continuing with the next trial")
                if output.strip():
                    print(output.rstrip())
                continue
            results[candidate].append(throughput)
            print(f"  throughput={throughput:.6f} seeds/sec")

    if args.dry_run:
        return None, seed_cursor
    return print_results(sweep, results, args.efficiency / 100.0), seed_cursor


def selected_sweeps(args: argparse.Namespace) -> list[Sweep]:
    selected = args.test or ["worker", "writer", "commit"]
    definitions = {
        "worker": Sweep("worker", "WORKER_THREADS", args.worker_values, False),
        "writer": Sweep("writer", "WRITER_THREADS", args.writer_values, True),
        "commit": Sweep("commit", "COMMIT_COUNT", args.commit_values, True),
    }
    return [definitions[name] for name in selected]


def warn_partial_batches(sweeps: Iterable[Sweep], args: argparse.Namespace) -> None:
    names = {sweep.name for sweep in sweeps}
    if "writer" in names:
        required = max(args.writer_values) * args.base_commit_count
        if args.seeds_per_trial < required:
            print(
                "warning: writer sweep may end with partial batches: "
                f"SEEDS_PER_TRIAL={args.seeds_per_trial}, required for one full "
                f"batch per maximum writer={required}"
            )
    if "commit" in names:
        required = max(args.commit_values) * args.base_writers
        if args.seeds_per_trial < required:
            print(
                "warning: commit sweep may end with partial batches: "
                f"SEEDS_PER_TRIAL={args.seeds_per_trial}, required for one full "
                f"maximum-size batch per writer={required}"
            )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--test",
        action="append",
        choices=("worker", "writer", "commit"),
        help="sweep one dimension; repeat for multiple dimensions (default: all)",
    )
    parser.add_argument(
        "--command",
        default="./inserter/target/release/dsp_seed_finder",
        help="command to benchmark; use a separate {docker_env} token for docker run/compose",
    )
    parser.add_argument("--worker-values", type=candidate_values, default=candidate_values("1,2,4,8,12,16,24,31"))
    parser.add_argument("--writer-values", type=candidate_values, default=candidate_values("1,2,4,8,12,16"))
    parser.add_argument("--commit-values", type=candidate_values, default=candidate_values("100,250,500,1000,2500,5000,10000"))
    parser.add_argument("--base-workers", type=positive_int, default=int(os.getenv("WORKER_THREADS", "8")))
    parser.add_argument("--base-writers", type=positive_int, default=int(os.getenv("WRITER_THREADS", "4")))
    parser.add_argument("--base-commit-count", type=positive_int, default=int(os.getenv("COMMIT_COUNT", "1000")))
    parser.add_argument("--channel-size", type=positive_int, default=int(os.getenv("CHANNEL_SIZE", "1000")))
    parser.add_argument("--seeds-per-trial", type=positive_int, default=10_000)
    parser.add_argument("--repetitions", type=positive_int, default=3)
    parser.add_argument("--efficiency", type=efficiency_percent, default=95.0, metavar="PERCENT")
    parser.add_argument("--order-seed", type=int, default=0, help="candidate randomization seed")
    parser.add_argument("--start-seed", type=int, help="first seed for trials; required for DB-writing tests")
    parser.add_argument("--allow-db-writes", action="store_true", help="acknowledge writer/commit trials insert persistent database rows")
    parser.add_argument(
        "--test-only",
        action="store_true",
        help="run without persistent pipeline or configuration changes",
    )
    parser.add_argument("--dry-run", action="store_true", help="print trial commands without running workloads or changing files")
    parser.add_argument("--apply-to", type=Path, metavar="ENV_FILE", help="offer to write confirmed recommendations to this env file")
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    sweeps = selected_sweeps(args)
    has_database_sweep = any(sweep.database_writes for sweep in sweeps)

    if args.test_only and args.apply_to is not None:
        parser.error("--test-only cannot be combined with --apply-to")
    if has_database_sweep and not args.dry_run and not args.test_only and not args.allow_db_writes:
        parser.error("writer/commit tests require --allow-db-writes")
    if has_database_sweep and not args.dry_run and not args.test_only and args.start_seed is None:
        parser.error("writer/commit tests require an explicit --start-seed")

    warn_partial_batches(sweeps, args)
    seed_cursor = args.start_seed if args.start_seed is not None else 0
    recommendations: dict[str, int] = {}
    with tempfile.TemporaryDirectory(prefix="dsp-tuning-") as checkpoint_directory:
        checkpoint_path = Path(checkpoint_directory)
        for sweep in sweeps:
            recommendation, seed_cursor = run_sweep(
                sweep, args, seed_cursor, checkpoint_path
            )
            if recommendation is not None:
                recommendations[sweep.env_name] = recommendation

    if args.dry_run:
        print("Dry run complete: 0 workloads executed and 0 files changed.")
        return 0
    if not recommendations:
        print("No recommendations available; 0 files changed.")
        return 1
    if args.test_only:
        print("Test-only mode complete: 0 configuration files changed.")
        return 0
    if args.apply_to is None:
        print("Recommendation-only mode: 0 configuration files changed.")
        for name, value in recommendations.items():
            print(f"export {name}={value}")
        return 0

    accepted = confirmed_updates(recommendations, args.apply_to)
    if accepted:
        update_env_file(args.apply_to, accepted)
        print(f"Wrote {len(accepted)} confirmed setting(s) to {args.apply_to}.")
    else:
        print("No settings accepted; 0 configuration files changed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
