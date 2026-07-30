# Vein Estimator Benchmark Record: 2026-07-30

## Purpose

Record the rejected vein-estimator experiment, its running configuration, and
the production decision for future comparison. This is a benchmark and
debugging record, not a production performance claim.

## Running Configuration

| Setting | Value |
| --- | --- |
| Calibration corpus | Seeds 0 through 999 |
| Held-out corpus | Seeds 1000000 through 1000999; not opened |
| Performance corpus | Seeds 2000000 through 2000199 |
| Corpus SHA-256 | `6d8c5f92824ecdaa0ad0298c2f418ca38a59bb6cd23ff0b7c49d3b2c8c434a21` |
| Stars per seed | 64 |
| Resource multiplier | 1.0 |
| Builder | `rust:1.97.0@sha256:b92b8c8574f8f3b207fcb0912fb3e2de4041580b5934d90312d53938c9a038a9` |
| Target | `x86_64-unknown-linux-musl` |
| Processor | Intel Xeon Platinum 8370C at 2.80 GHz |
| Available processors | 2 logical processors, 1 physical core |
| Benchmark workers | 1 |
| Processor set | 0 |
| Comparison order | A, B, B, A for three rounds |
| Disabled during comparison | UI, logs, checkpoints, database writes |

## Provenance

| Item | Source revision | SHA-256 |
| --- | --- | --- |
| Historical fast baseline | `0df70ce645d4573ea135ea179a1261890851ec00` | `2b078c659e56a4ffaa31e905170a3aa96c363294db28f87a2241db76e2d4ba5e` |
| Exact baseline | `59232b78b5e8065b5697fe6bf6cd253c3a15f9e9` | `17445fbe6f040420e974e197a7d86e841c03263079daeaaaeaa190113552e5fa` |
| Validation tooling | `5eda1b60c58bb97a7ea836698f2db2eb910fac86` | exporter: `6e6d8cb6c084a754fbc7634989bf4539b45f3615ccbacd8f16636023cd85ec2b` |

The production generator's runtime behavior remains the exact baseline.

## Results

| Measurement | Historical fast median | Exact median | Change |
| --- | ---: | ---: | ---: |
| Initial comparison | 248.415 seeds/s | 4.736 seeds/s | -98.09% |
| Repeated comparison | 248.325 seeds/s | 4.691 seeds/s | -98.11% |

The 5% elapsed-time-overhead limit requires a candidate throughput of at least
236.500 seeds/s. The exact generator is 52.94 times slower than the historical
fast baseline.

| Candidate | Overall WAPE | P95 absolute relative error | Outcome |
| --- | ---: | ---: | --- |
| Historical midpoint | 10.96% | 91.40% | Rejected |
| Spacing replay | 6.64% | 57.91% | Rejected |

The spacing replay also exceeded the 15% per-ore WAPE limit for Kimberlite ore,
fractal silicon, and organic crystal. The required limits are overall WAPE at
most 10%, WAPE at most 15% for every ore, P95 absolute relative error at most
25%, and existence precision and recall at least 99% for every ore.

## Decision And Lessons

No estimator was shipped. The spacing replay improved overall WAPE, but replaying
group counts and spacing without terrain did not control tail error or rare-ore
error. The held-out corpus remains protected, and candidate performance was not
measured because calibration already failed.

Future estimator work should use the same corpus manifest, source/exporter
provenance, calibration gates, held-out protection, and performance comparison.

## Detailed Records

- [Study and reproduction guide](../../docs/vein-estimator-study.md)
- [Initial raw comparison](2026-07-30-vein-baselines.json)
- [Repeated raw comparison](2026-07-30-vein-final-comparison.json)
- [Historical midpoint calibration](2026-07-30-vein-calibration-midpoint.json)
- [Spacing replay calibration](2026-07-30-vein-calibration-spacing.json)
