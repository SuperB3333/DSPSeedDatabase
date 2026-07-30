#!/usr/bin/env python3
"""Fit fixed-point theme and ore scales on a calibration oracle."""

from __future__ import annotations

import argparse
import csv
import json
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path

from evaluate_vein_accuracy import (
    DEFAULT_CORPUS_MANIFEST,
    ORE_NAMES,
    file_sha256,
    load_seed_set,
    validate_seed_values,
)


DENOMINATOR = 65_536


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--corpus-manifest", type=Path, default=DEFAULT_CORPUS_MANIFEST)
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--exporter", required=True, type=Path)
    return parser.parse_args()


def weighted_median_scale(samples: list[tuple[int, int]]) -> int:
    if not samples:
        return DENOMINATOR
    ordered = sorted(samples, key=lambda pair: pair[1] / pair[0])
    total_weight = sum(estimate for estimate, _ in ordered)
    cumulative = 0
    for estimate, actual in ordered:
        cumulative += estimate
        if cumulative * 2 >= total_weight:
            return max(1, (actual * DENOMINATOR + estimate // 2) // estimate)
    raise AssertionError("weighted median was not found")


def main() -> None:
    args = parse_args()
    if not args.exporter.is_file():
        raise ValueError(f"exporter does not exist: {args.exporter}")
    seed_definition, corpus_sha256 = load_seed_set(
        args.corpus_manifest, "calibration"
    )
    samples: dict[tuple[int, int], list[tuple[int, int]]] = defaultdict(list)
    seeds: set[int] = set()
    with args.input.open(newline="", encoding="ascii") as source:
        for row in csv.DictReader(source):
            seeds.add(int(row["seed"]))
            if row["gas"] == "1":
                continue
            theme_id = int(row["theme_id"])
            for ore_index, ore in enumerate(ORE_NAMES):
                estimate = int(row[f"estimate_{ore}"])
                if estimate > 0:
                    samples[(theme_id, ore_index)].append(
                        (estimate, int(row[f"actual_{ore}"]))
                    )
    validate_seed_values(seeds, seed_definition)

    theme_scales = {
        str(theme_id): [
            weighted_median_scale(samples[(theme_id, ore_index)])
            for ore_index in range(len(ORE_NAMES))
        ]
        for theme_id in range(1, 26)
    }
    artifact = {
        "schema_version": 2,
        "created_utc": datetime.now(timezone.utc).isoformat(),
        "method": "weighted median of actual divided by historical estimate",
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
        },
        "denominator": DENOMINATOR,
        "theme_scales": theme_scales,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(artifact, indent=2) + "\n", encoding="ascii")


if __name__ == "__main__":
    main()
