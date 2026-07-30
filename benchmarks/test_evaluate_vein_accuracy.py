#!/usr/bin/env python3

import unittest

from evaluate_vein_accuracy import (
    ORE_NAMES,
    Accuracy,
    accuracy_requirement_results,
    percentile_nearest_rank,
    scale_estimate,
    validate_seed_values,
)
from fit_vein_scales import DENOMINATOR, weighted_median_scale
from run_comparison import summarize


class AccuracyTest(unittest.TestCase):
    def test_reports_amount_and_existence_errors(self) -> None:
        accuracy = Accuracy()
        accuracy.add(90, 100)
        accuracy.add(20, 0)
        accuracy.add(0, 50)
        accuracy.add(0, 0)

        result = accuracy.result()
        self.assertAlmostEqual(
            result["all_amounts"]["weighted_absolute_percentage_error"],
            80 / 150,
        )
        self.assertEqual(result["null_reference"]["correct"], 1)
        self.assertEqual(result["existence"]["true_positive"], 1)
        self.assertEqual(result["existence"]["false_positive"], 1)
        self.assertEqual(result["existence"]["false_negative"], 1)

    def test_undefined_existence_metrics_fail_the_accuracy_gate(self) -> None:
        accuracy = Accuracy()
        accuracy.add(0, 0)
        result = accuracy.result()
        by_ore = {ore: result for ore in ORE_NAMES}

        self.assertIsNone(result["existence"]["precision"])
        self.assertIsNone(result["existence"]["recall"])
        self.assertFalse(
            accuracy_requirement_results(result, by_ore)[
                "each_ore_existence_precision_and_recall_at_least_0_99"
            ]
        )

    def test_percentile_uses_nearest_rank(self) -> None:
        self.assertEqual(percentile_nearest_rank(list(range(1, 21)), 0.95), 19)

    def test_seed_values_must_match_the_locked_range(self) -> None:
        definition = {"start_seed": 10, "end_seed_exclusive": 13}
        validate_seed_values({10, 11, 12}, definition)
        with self.assertRaises(ValueError):
            validate_seed_values({10, 12}, definition)

    def test_fixed_point_scale_rounds_and_clamps(self) -> None:
        scales = {7: [98_304] * 14}
        self.assertEqual(scale_estimate(3, 7, 0, 65_536, scales), 5)
        self.assertEqual(
            scale_estimate(2_147_483_647, 7, 0, 65_536, scales),
            2_147_483_647,
        )

    def test_weighted_median_scale_uses_estimate_weights(self) -> None:
        self.assertEqual(
            weighted_median_scale([(2, 2), (10, 20), (2, 6)]),
            2 * DENOMINATOR,
        )

    def test_performance_gate_uses_elapsed_time_overhead(self) -> None:
        passing = [
            {"variant": "baseline", "throughput_seeds_per_second": 100.0},
            {"variant": "candidate", "throughput_seeds_per_second": 96.0},
        ]
        failing = [
            {"variant": "baseline", "throughput_seeds_per_second": 100.0},
            {"variant": "candidate", "throughput_seeds_per_second": 95.0},
        ]

        self.assertTrue(
            summarize(passing, "candidate")["performance_requirement"]["passed"]
        )
        self.assertFalse(
            summarize(failing, "candidate")["performance_requirement"]["passed"]
        )


if __name__ == "__main__":
    unittest.main()
