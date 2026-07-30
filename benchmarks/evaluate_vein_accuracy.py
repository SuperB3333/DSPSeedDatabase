#!/usr/bin/env python3
"""Measure planet ore estimates against exact vein-generation output."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path


ORE_NAMES = (
    "iron",
    "copper",
    "silicium",
    "titanium",
    "stone",
    "coal",
    "oil",
    "fireice",
    "diamond",
    "fractal",
    "crysrub",
    "grat",
    "bamboo",
    "mag",
)
MAX_I32 = 2_147_483_647
DEFAULT_CORPUS_MANIFEST = Path(__file__).with_name("vein_estimator_corpora.json")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--scales", type=Path)
    parser.add_argument("--seed-set", required=True)
    parser.add_argument("--estimate-prefix", default="estimate")
    parser.add_argument("--corpus-manifest", type=Path, default=DEFAULT_CORPUS_MANIFEST)
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--exporter", required=True, type=Path)
    parser.add_argument("--allow-held-out", action="store_true")
    return parser.parse_args()


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def percentile_nearest_rank(values: list[float], fraction: float) -> float | None:
    if not values:
        return None
    values.sort()
    return values[max(0, math.ceil(fraction * len(values)) - 1)]


@dataclass
class Accuracy:
    count: int = 0
    reference_sum: int = 0
    predicted_sum: int = 0
    absolute_error_sum: int = 0
    signed_error_sum: int = 0
    non_null_count: int = 0
    non_null_absolute_error_sum: int = 0
    non_null_signed_error_sum: int = 0
    null_count: int = 0
    correct_null_count: int = 0
    true_positive: int = 0
    false_positive: int = 0
    false_negative: int = 0
    relative_errors: list[float] = field(default_factory=list)

    def add(self, predicted: int, actual: int) -> None:
        error = predicted - actual
        absolute_error = abs(error)
        self.count += 1
        self.reference_sum += actual
        self.predicted_sum += predicted
        self.absolute_error_sum += absolute_error
        self.signed_error_sum += error
        if actual > 0:
            self.non_null_count += 1
            self.non_null_absolute_error_sum += absolute_error
            self.non_null_signed_error_sum += error
            self.relative_errors.append(absolute_error / actual)
            if predicted > 0:
                self.true_positive += 1
            else:
                self.false_negative += 1
        else:
            self.null_count += 1
            if predicted > 0:
                self.false_positive += 1
            else:
                self.correct_null_count += 1

    def result(self) -> dict[str, object]:
        predicted_positive = self.true_positive + self.false_positive
        reference_positive = self.true_positive + self.false_negative
        precision = self.true_positive / predicted_positive if predicted_positive else None
        recall = self.true_positive / reference_positive if reference_positive else None
        return {
            "all_amounts": {
                "count": self.count,
                "reference_sum": self.reference_sum,
                "predicted_sum": self.predicted_sum,
                "mean_absolute_error": self.absolute_error_sum / self.count,
                "weighted_absolute_percentage_error": (
                    self.absolute_error_sum / self.reference_sum
                    if self.reference_sum
                    else 0.0
                ),
                "weighted_signed_bias": (
                    self.signed_error_sum / self.reference_sum
                    if self.reference_sum
                    else 0.0
                ),
            },
            "non_null_reference": {
                "count": self.non_null_count,
                "mean_absolute_error": (
                    self.non_null_absolute_error_sum / self.non_null_count
                    if self.non_null_count
                    else 0.0
                ),
                "mean_absolute_relative_error": (
                    sum(self.relative_errors) / self.non_null_count
                    if self.non_null_count
                    else 0.0
                ),
                "p95_absolute_relative_error": percentile_nearest_rank(
                    self.relative_errors, 0.95
                ),
                "mean_signed_error": (
                    self.non_null_signed_error_sum / self.non_null_count
                    if self.non_null_count
                    else 0.0
                ),
            },
            "null_reference": {
                "count": self.null_count,
                "correct": self.correct_null_count,
                "false_positive": self.false_positive,
                "accuracy": (
                    self.correct_null_count / self.null_count
                    if self.null_count
                    else 1.0
                ),
            },
            "existence": {
                "true_positive": self.true_positive,
                "false_positive": self.false_positive,
                "false_negative": self.false_negative,
                "precision": precision,
                "recall": recall,
            },
        }


def load_scales(path: Path | None) -> tuple[int, dict[int, list[int]], str | None]:
    if path is None:
        return 1, {}, None
    artifact = json.loads(path.read_text(encoding="ascii"))
    denominator = int(artifact["denominator"])
    scales = {
        int(theme): [int(value) for value in values]
        for theme, values in artifact["theme_scales"].items()
    }
    for theme, values in scales.items():
        if len(values) != len(ORE_NAMES):
            raise ValueError(f"theme {theme} does not have {len(ORE_NAMES)} scales")
    return denominator, scales, file_sha256(path)


def load_seed_set(
    manifest_path: Path, seed_set: str, allow_held_out: bool = False
) -> tuple[dict[str, int], str]:
    manifest = json.loads(manifest_path.read_text(encoding="ascii"))
    if seed_set == "held_out_validation" and not allow_held_out:
        raise ValueError("held-out validation requires --allow-held-out")
    try:
        definition = manifest["seed_sets"][seed_set]
    except KeyError as error:
        raise ValueError(f"unknown seed set: {seed_set}") from error
    start = int(definition["start_seed"])
    end = int(definition["end_seed_exclusive"])
    if start >= end:
        raise ValueError(f"invalid seed range for {seed_set}")
    return {"start_seed": start, "end_seed_exclusive": end}, file_sha256(
        manifest_path
    )


def validate_seed_values(seeds: set[int], definition: dict[str, int]) -> None:
    expected = set(
        range(definition["start_seed"], definition["end_seed_exclusive"])
    )
    if seeds == expected:
        return
    missing = sorted(expected - seeds)[:5]
    unexpected = sorted(seeds - expected)[:5]
    raise ValueError(
        f"input does not match seed set; missing={missing}, unexpected={unexpected}"
    )


def scale_estimate(
    estimate: int,
    theme_id: int,
    ore_index: int,
    denominator: int,
    scales: dict[int, list[int]],
) -> int:
    scale = scales.get(theme_id, [denominator] * len(ORE_NAMES))[ore_index]
    value = (estimate * scale + denominator // 2) // denominator
    return min(value, MAX_I32)


def evaluate(
    input_path: Path,
    denominator: int,
    scales: dict[int, list[int]],
    estimate_prefix: str,
) -> tuple[set[int], int, dict[str, object], dict[str, dict[str, object]]]:
    overall = Accuracy()
    by_ore = {ore: Accuracy() for ore in ORE_NAMES}
    seeds: set[int] = set()
    planet_count = 0
    with input_path.open(newline="", encoding="ascii") as source:
        reader = csv.DictReader(source)
        for row in reader:
            seeds.add(int(row["seed"]))
            planet_count += 1
            gas = row["gas"] == "1"
            theme_id = int(row["theme_id"])
            for ore_index, ore in enumerate(ORE_NAMES):
                estimate = int(row[f"{estimate_prefix}_{ore}"])
                actual = int(row[f"actual_{ore}"])
                predicted = 0 if gas else scale_estimate(
                    estimate, theme_id, ore_index, denominator, scales
                )
                by_ore[ore].add(predicted, actual)
                overall.add(predicted, actual)
    return (
        seeds,
        planet_count,
        overall.result(),
        {ore: values.result() for ore, values in by_ore.items()},
    )


def accuracy_requirement_results(
    overall: dict[str, object], by_ore: dict[str, dict[str, object]]
) -> dict[str, object]:
    overall_wape = overall["all_amounts"]["weighted_absolute_percentage_error"]
    overall_p95 = overall["non_null_reference"]["p95_absolute_relative_error"]
    all_ore_wape = all(
        values["all_amounts"]["weighted_absolute_percentage_error"] <= 0.15
        for values in by_ore.values()
    )
    all_existence = all(
        values["existence"]["precision"] is not None
        and values["existence"]["precision"] >= 0.99
        and values["existence"]["recall"] is not None
        and values["existence"]["recall"] >= 0.99
        for values in by_ore.values()
    )
    return {
        "overall_wape_at_most_0_10": overall_wape <= 0.10,
        "each_ore_wape_at_most_0_15": all_ore_wape,
        "overall_non_null_p95_are_at_most_0_25": overall_p95 is not None
        and overall_p95 <= 0.25,
        "each_ore_existence_precision_and_recall_at_least_0_99": all_existence,
        "passed": overall_wape <= 0.10
        and all_ore_wape
        and overall_p95 is not None
        and overall_p95 <= 0.25
        and all_existence,
    }


def main() -> None:
    args = parse_args()
    if not args.exporter.is_file():
        raise ValueError(f"exporter does not exist: {args.exporter}")
    seed_definition, corpus_sha256 = load_seed_set(
        args.corpus_manifest, args.seed_set, args.allow_held_out
    )
    denominator, scales, scale_sha256 = load_scales(args.scales)
    seeds, planet_count, overall, by_ore = evaluate(
        args.input, denominator, scales, args.estimate_prefix
    )
    validate_seed_values(seeds, seed_definition)
    artifact = {
        "schema_version": 2,
        "created_utc": datetime.now(timezone.utc).isoformat(),
        "seed_set": args.seed_set,
        "estimate_prefix": args.estimate_prefix,
        "corpus": {
            "manifest": str(args.corpus_manifest),
            "sha256": corpus_sha256,
            **seed_definition,
        },
        "provenance": {
            "source_revision": args.source_revision,
            "exporter": str(args.exporter),
            "exporter_sha256": file_sha256(args.exporter),
        },
        "input": {
            "path": str(args.input),
            "sha256": file_sha256(args.input),
            "seed_count": len(seeds),
            "planet_count": planet_count,
        },
        "scales": {
            "path": str(args.scales) if args.scales else None,
            "sha256": scale_sha256,
            "denominator": denominator,
        },
        "overall": overall,
        "by_ore": by_ore,
    }
    artifact["accuracy_requirements"] = accuracy_requirement_results(overall, by_ore)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(artifact, indent=2) + "\n", encoding="ascii")
    print(
        json.dumps(
            {
                "overall": overall,
                "accuracy_requirements": artifact["accuracy_requirements"],
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
