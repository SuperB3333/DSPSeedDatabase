# Planet Vein Estimator Validation Tools

## Purpose

This change adds developer tooling for exporting exact planet vein amounts,
measuring deterministic estimator candidates, and comparing process throughput.
It does not change production generation. The current study records a rejected
candidate so future work can reproduce the baselines and use the protected
held-out corpus correctly.

## Study Outcome

The release gate did not pass. The production generator still uses exact terrain
and vein placement. This decision prevents the release of an inaccurate
estimator.

The best fast calibration candidate had 6.64% weighted absolute percentage
error (WAPE). However, its P95 absolute relative error was 57.91%. Kimberlite
ore, fractal silicon, and organic crystal also had more than 15% WAPE. The
required limits are 10% overall WAPE, 15% WAPE for each ore, and 25% P95 error.

The held-out seed set was not opened. A candidate must pass all calibration
gates before the held-out test. Thus, the held-out set remains valid for future
work.

## Tooling

- `inserter/src/bin/vein_accuracy.rs` exports historical midpoint estimates,
  the spacing candidate, and exact actual amounts without a database.
- `benchmarks/evaluate_vein_accuracy.py` validates a locked seed corpus and
  reports amount, NULL, and existence metrics for an exporter CSV.
- `benchmarks/fit_vein_scales.py` generates fixed-point theme and ore scales
  from the calibration corpus.
- `benchmarks/run_comparison.py` records controlled A-B-B-A process runs and
  reports the maximum 5% elapsed-time-overhead requirement.

The exporter and evaluator reject the held-out seed set unless the caller passes
`--allow-held-out`. The held-out range must only be opened after all calibration
accuracy gates pass.

## Workflow

1. Build `vein_accuracy` with the locked dependencies and the musl target.
2. Export the calibration range, then evaluate an estimator column against the
   exact `actual_*` columns. The evaluator verifies that the CSV contains every
   seed in the selected corpus and records hashes for the corpus, input, and
   exporter.
3. Optionally fit fixed-point theme and ore scales from the historical
   `estimate_*` columns, then re-evaluate with the generated scale artifact.
4. Open the held-out corpus only if every calibration accuracy gate passes.
5. Run the process comparison on the performance corpus. A candidate must meet
   the elapsed-time-overhead requirement as well as the accuracy requirements.

The evaluator reports the following accuracy requirements: overall WAPE at most
10%; WAPE at most 15% for every ore; non-null-reference P95 absolute relative
error at most 25%; and precision and recall at least 99% for the existence of
every ore. The comparison tool requires at most 5% elapsed-time overhead, which
is equivalent to candidate throughput of at least baseline throughput divided by
1.05.

## Test Inputs

The file [`vein_estimator_corpora.json`](../benchmarks/vein_estimator_corpora.json)
defines three separate seed sets. Its SHA-256 value is
`6d8c5f92824ecdaa0ad0298c2f418ca38a59bb6cd23ff0b7c49d3b2c8c434a21`.

| Set | Seeds | Use |
| --- | --- | --- |
| Calibration | 0 through 999 | Model measurements |
| Held-out validation | 1000000 through 1000999 | Not opened |
| Performance | 2000000 through 2000199 | Controlled process tests |

Each seed has 64 stars and a resource multiplier of 1.0.

## Test System

| Item | Value |
| --- | --- |
| Processor | Intel Xeon Platinum 8370C at 2.80 GHz |
| Available processors | 2 logical processors, 1 physical core |
| Memory | 8,270,495,744 bytes |
| Kernel | Linux 6.17.0-1021-azure |
| Docker | 29.6.1 |
| Rust | 1.97.0 |
| Cargo | 1.97.0 |
| Target | `x86_64-unknown-linux-musl` |
| Worker count | 1 |
| Processor set | 0 |

The build used the locked dependencies and the release profile. The builder was
`rust:1.97.0@sha256:b92b8c8574f8f3b207fcb0912fb3e2de4041580b5934d90312d53938c9a038a9`.

## Baselines

The fast baseline is commit
`0df70ce645d4573ea135ea179a1261890851ec00`. This is the last buildable source
version before precise planet ores entered the main branch. It calculates
midpoint estimates and does not simulate terrain or vein placement. Its
executable SHA-256 value is
`2b078c659e56a4ffaa31e905170a3aa96c363294db28f87a2241db76e2d4ba5e`.

The exact baseline is commit
`59232b78b5e8065b5697fe6bf6cd253c3a15f9e9`. Its executable SHA-256 value is
`17445fbe6f040420e974e197a7d86e841c03263079daeaaaeaa190113552e5fa`.

The test used the sequence `A, B, B, A` for three rounds. It disabled the user
interface, logs, checkpoints, and database writes.

| Test | Fast median | Exact median | Exact change |
| --- | ---: | ---: | ---: |
| Initial | 248.415 seeds/s | 4.736 seeds/s | -98.09% |
| Final repeat | 248.325 seeds/s | 4.691 seeds/s | -98.11% |

For a maximum 5% increase in elapsed time, the final throughput limit is
236.500 seeds/s (`248.325 / 1.05`). The exact implementation is 52.94 times
slower than the fast baseline. The spacing candidate did not receive a release
performance test because it failed the calibration accuracy gates first.

Raw results are in
[`2026-07-30-vein-baselines.json`](../benchmarks/results/2026-07-30-vein-baselines.json)
and
[`2026-07-30-vein-final-comparison.json`](../benchmarks/results/2026-07-30-vein-final-comparison.json).

## Candidate Algorithm

The historical midpoint candidate uses theme spot count, patch count, vein
opacity, star resource coefficients, and rare-ore rolls. It does not use
terrain. Its calibration result was 10.96% WAPE and 91.40% P95 relative error.

The best fast candidate also replays these operations:

1. It starts the placement random-number stream from the planet seed.
2. It applies the exact minus-one, zero, or plus-one group-count roll.
3. It creates candidate group directions.
4. It rejects directions that are too close to an earlier group.
5. It accepts terrain without a height query.
6. It scales the midpoint amount by accepted groups divided by nominal groups.

The candidate uses a fixed stack array for 512 group centers. It does not
allocate center storage. The file `vein_accuracy.rs` contains this calibration
implementation. Production does not call it.

The exact group-count roll is important. For a nominal count of two, the exact
result commonly has one, two, or three groups. A midpoint cannot predict this
three-mode result. Terrain rejection also changes the random-number position
for later ores.

Exploratory terrain candidates also failed the accuracy limits and were removed
before the final measurements.

## Calibration Accuracy

The table reports the best fast candidate. MAE includes null reference amounts
as zero. Mean absolute relative error and P95 absolute relative error use
non-null reference amounts. Signed bias is
`sum(estimate - exact) / sum(exact)`.

| Ore | MAE | WAPE | Mean absolute relative error | P95 absolute relative error | Signed bias | Precision | Recall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Iron | 829682 | 3.55% | 3.98% | 11.84% | +2.27% | 100.00% | 100.00% |
| Copper | 1452015 | 5.90% | 12.02% | 50.68% | +2.54% | 100.00% | 100.00% |
| Silicon | 825620 | 7.68% | 18.94% | 90.20% | +0.50% | 100.00% | 100.00% |
| Titanium | 1594175 | 7.09% | 8.93% | 32.68% | +1.83% | 100.00% | 100.00% |
| Stone | 950954 | 6.45% | 16.83% | 67.99% | +0.70% | 100.00% | 100.00% |
| Coal | 462329 | 14.76% | 19.03% | 67.32% | +4.70% | 100.00% | 100.00% |
| Oil | 10295 | 4.67% | 4.97% | 13.76% | +0.15% | 100.00% | 100.00% |
| Fire ice | 355258 | 9.63% | 14.91% | 62.88% | +0.45% | 100.00% | 100.00% |
| Kimberlite ore | 288294 | 17.20% | 18.47% | 67.84% | +0.38% | 100.00% | 100.00% |
| Fractal silicon | 64583 | 15.45% | 16.12% | 66.13% | +1.36% | 100.00% | 100.00% |
| Organic crystal | 100777 | 27.08% | 29.27% | 119.92% | +6.71% | 100.00% | 100.00% |
| Optical grating crystal | 134323 | 12.98% | 15.19% | 62.86% | -0.16% | 100.00% | 100.00% |
| Spiniform stalagmite crystal | 37519 | 7.32% | 18.06% | 82.92% | +1.42% | 100.00% | 100.00% |
| Unipolar magnet | 5420 | 14.59% | 17.66% | 67.80% | +1.29% | 100.00% | 100.00% |

The source uses internal names for some ores. The raw result keeps these names:
`crysrub`, `grat`, `bamboo`, and `mag`. The full machine-readable result is
[`2026-07-30-vein-calibration-spacing.json`](../benchmarks/results/2026-07-30-vein-calibration-spacing.json).

There were 1,394,252 non-null reference amounts and 2,025,094 null reference
amounts. The candidate had no false presence and no missed presence. Precision
and recall were 100% for each ore.

## Verification

- The production generator runtime behavior has no change from commit `59232b78`.
- Two branch runs and one source-commit run for seeds 0 through 9 were byte equal.
- The output SHA-256 value was `940014de4f0352f8577bec3a351523108a0579186837b5100a30413b96382f00`.
- The branch executable hashes equal the exact baseline executable hashes.
- The database schema SHA-256 value stayed `b9c2d2a23b81af92dfd58188cf00664eca84c202ed4a647055c0eb5c3879a78d`.
- The schema still has one nullable column for each ore. It has no minimum,
  maximum, or average ore columns.
- A focused test checks binary SQL NULL fields for gas planets and absent rocky
  planet ores.
- Rust and Python tests pass.

## Result Artifacts

- [`2026-07-30-vein-estimator-benchmark.md`](../benchmarks/results/2026-07-30-vein-estimator-benchmark.md)
  is the concise timeline record of configuration, provenance, results, and the
  decision not to ship an estimator.
- [`2026-07-30-vein-baselines.json`](../benchmarks/results/2026-07-30-vein-baselines.json)
  records the initial historical-fast versus exact A-B-B-A comparison.
- [`2026-07-30-vein-final-comparison.json`](../benchmarks/results/2026-07-30-vein-final-comparison.json)
  records the repeated comparison and the computed elapsed-time requirement.
- [`2026-07-30-vein-calibration-midpoint.json`](../benchmarks/results/2026-07-30-vein-calibration-midpoint.json)
  records the historical midpoint candidate's calibration result.
- [`2026-07-30-vein-calibration-spacing.json`](../benchmarks/results/2026-07-30-vein-calibration-spacing.json)
  records the rejected spacing candidate's calibration result.

The calibration results are schema version 2. They record the corpus, input,
source revision, exporter path and hash, estimator prefix, metrics, and gate
outcome. The performance artifacts contain the alternating run order, host and
workload configuration, executable hashes, raw process measurements, and the
computed overhead requirement.

## Reproduction

Build the oracle exporter with the pinned builder and musl target:

```bash
docker run --rm -v "$PWD/inserter:/app" -w /app \
  rust:1.97.0@sha256:b92b8c8574f8f3b207fcb0912fb3e2de4041580b5934d90312d53938c9a038a9 \
  sh -c 'rustup target add x86_64-unknown-linux-musl && cargo build --release --locked --target x86_64-unknown-linux-musl --bin vein_accuracy'
```

Export and evaluate a calibration candidate. Substitute the source revision that
produced the exporter when creating a new artifact:

```bash
inserter/target/x86_64-unknown-linux-musl/release/vein_accuracy \
  0 1000 /tmp/vein-calibration.csv

python3 benchmarks/evaluate_vein_accuracy.py \
  --input /tmp/vein-calibration.csv \
  --output /tmp/vein-calibration-result.json \
  --estimate-prefix spacing \
  --seed-set calibration \
  --source-revision "$(git rev-parse HEAD)" \
  --exporter inserter/target/x86_64-unknown-linux-musl/release/vein_accuracy
```

To fit scales for the historical estimate, then evaluate the scaled estimate:

```bash
python3 benchmarks/fit_vein_scales.py \
  --input /tmp/vein-calibration.csv \
  --output /tmp/vein-scales.json \
  --source-revision "$(git rev-parse HEAD)" \
  --exporter inserter/target/x86_64-unknown-linux-musl/release/vein_accuracy

python3 benchmarks/evaluate_vein_accuracy.py \
  --input /tmp/vein-calibration.csv \
  --output /tmp/vein-scaled-result.json \
  --scales /tmp/vein-scales.json \
  --estimate-prefix estimate \
  --seed-set calibration \
  --source-revision "$(git rev-parse HEAD)" \
  --exporter inserter/target/x86_64-unknown-linux-musl/release/vein_accuracy
```

Measure an eligible candidate against a baseline with the locked performance
corpus. The candidate must honor the benchmark environment supplied by the
comparison script.

```bash
python3 benchmarks/run_comparison.py \
  --baseline /path/to/baseline/dsp_seed_finder \
  --candidate /path/to/candidate/dsp_seed_finder \
  --candidate-name candidate \
  --output /tmp/vein-performance.json \
  --start-seed 2000000 --end-seed 2000200 \
  --workers 1 --rounds 3 --cpus 0
```

Do not use the held-out range until a calibration candidate passes all accuracy
gates. Both the exporter and evaluator require an explicit `--allow-held-out`
flag for the held-out range.
