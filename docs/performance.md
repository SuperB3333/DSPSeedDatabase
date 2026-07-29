# Generation Pipeline Performance

## Purpose

This document gives the version 0.7.0 performance results, test method, and
recorded optimization experiments.

Throughput is the primary performance measure. Wall time is elapsed clock time.
The user central processing unit (CPU) time and the system CPU time measure
processor use.
Minor page faults count memory accesses that the operating system resolves
without disk access. Peak resident set size (RSS) measures peak physical memory
use.

The test did not measure power. The virtual machine does not supply a stable
power counter. The test did not collect hardware performance-counter data.

## Test System

| Item | Value |
| --- | --- |
| Date | 2026-07-29 |
| Processor | Intel Xeon Platinum 8370C at 2.80 GHz |
| Available processors | 2 logical processors, 1 physical core |
| Memory | 7.7 GiB |
| Kernel | Linux 6.17.0-1021-azure |
| Docker | 29.6.1 |
| Rust | 1.97.0 |
| Target | `x86_64-unknown-linux-musl` |
| Baseline source | `c0ed162ddf820b8d4ada0127b597cd64a8e35a88` |
| Baseline executable | `74e96f68daac80a35d4296fd06204d8e4c393785d493c5414ff5679d56f5747c` |
| Optimized executable | `b1d3c5d7216848b9fd73cccc36d6e9142574d8ac5d9b0e1e4dbeed4fcc83a31a` |

The executable values are 256-bit Secure Hash Algorithm (SHA-256) digests.

## Controlled Method

The controlled comparison processed seeds from 0 to 99. Each process used one
generator worker and one output sink. The output sink discarded generated
data. The process ran on logical processor 0. The test disabled logs,
checkpoints, and the terminal user interface (TUI). The process did not read
from or write to a database.

The benchmark measured executable run time only. It did not measure build time.
It ran the executable sequence `A, B, B, A` three times. This sequence gave six
baseline samples and six optimized samples.

The script calculates the median and median absolute deviation (MAD). The script
uses 20,000 bootstrap samples and bootstrap random seed 0. The script calculates
a 95% confidence interval for the paired change in median throughput.

Run the comparison with this command:

```bash
python3 benchmarks/run_comparison.py \
  --baseline /path/to/baseline/dsp_seed_finder \
  --candidate /path/to/optimized/dsp_seed_finder \
  --candidate-name optimized-0.7.0 \
  --output benchmarks/results/local.json \
  --start-seed 0 \
  --end-seed 100 \
  --workers 1 \
  --rounds 3 \
  --cpus 0
```

The script requires GNU `time` and `taskset`.

## Result

| Measure | Baseline median | Version 0.7.0 median | Change |
| --- | ---: | ---: | ---: |
| Throughput | 2.649 seeds/s | 3.668 seeds/s | +38.5% |
| Wall time for 100 seeds | 37.750 s | 27.265 s | -27.8% |
| User CPU time | 28.655 s | 25.680 s | -10.4% |
| System CPU time | 6.730 s | 0.110 s | -98.4% |
| Minor page faults | 3,320,943 | 7,012 | -99.8% |
| Peak RSS | 5,970 KiB | 5,654 KiB | -5.3% |

The paired throughput gain is 38.0%. Its bootstrap 95% interval is 32.0% to
39.5%. The throughput MAD is 0.022 seeds/s for the baseline and 0.013 seeds/s
for version 0.7.0.

The machine-readable raw result is
[`benchmarks/results/2026-07-29-final.json`](../benchmarks/results/2026-07-29-final.json).

## Profile Result

The baseline software profile had these sample percentages:

- 45.4% to three-dimensional simplex noise.
- 15.6% to page-fault handling.
- 10.6% to terrain-height interpolation.

The initial 100-seed run caused 3,319,980 minor page faults. The pooled terrain
cache reduced this value to 7,619 in the first cache experiment.

After the memory changes, 56.2% of samples occurred in simplex noise. A total
of 11.0% of samples occurred in terrain interpolation.

The virtual machine did not supply cycles, instructions, branches, or cache
misses from `perf`. The test used software sampling and wall measurements.

## Optimization Decisions

| Experiment | Decision | Measured result |
| --- | --- | --- |
| Reuse a compact terrain cache for each worker | Keep | +27.14% median throughput. The 95% interval was +16.58% to +31.35%. |
| Compare vein spacing before terrain | Keep | +5.14%. The interval was +2.62% to +8.39%. |
| Store simplex permutations as `u8` | Keep | +2.95%. The interval was +0.91% to +7.56%. |
| Fuse algorithm 12 noise passes | Revert | +0.20%. The interval was -4.76% to +0.75%. |
| Reuse the duplicate noise result in algorithm 9 | Keep | +0.75% median throughput. The 95% interval was -5.15% to +5.00%. |
| Make algorithm 10 turbulence conditional | Revert | -0.92%. The interval was -2.96% to +4.26%. |
| Do not calculate unused detail noise in algorithms 5 and 6 | Keep | +0.38% median throughput. The 95% interval was -1.69% to +7.97%. |
| Reject algorithm 4 craters with partial distance | Revert | -1.84%. The interval was -8.99% to -0.18%. |
| Wait for the first writer item | Keep for reliability | -1.49% median throughput. The 95% interval was -6.72% to +0.87%. |
| Reduce the default writer count and queue capacity | Keep | +0.40% throughput on 500 seeds. |
| Increase the adaptive worker default | Keep | On 500 seeds, 32 workers had 10.6% higher throughput than 8 workers. |

The machine-readable decision list is
[`benchmarks/results/2026-07-29-experiments.json`](../benchmarks/results/2026-07-29-experiments.json).

## PostgreSQL Scaling

PostgreSQL accepted the binary COPY data in the database test. PostgreSQL 16
and the generator shared the two logical processors. One writer used batches
of 64 seeds.

| Workers | Seeds | Throughput | Peak generator RSS |
| ---: | ---: | ---: | ---: |
| 2 | 500 | 4.538 seeds/s | 14,432 KiB |
| 8 | 500 | 5.962 seeds/s | 18,008 KiB |
| 16 | 500 | 6.179 seeds/s | 23,876 KiB |
| 32 | 500 | 6.595 seeds/s | 33,808 KiB |

The generation time is different for each static seed range. More workers
decrease the idle time at the end of the test. Four database writers did not
increase the 32-worker result. One writer is the default.

The baseline scale test processed 500 seeds. The test failed because the output
sink treated one second without input as a fatal error. Version 0.7.0 waits for
the first item. Version 0.7.0 writes a non-full batch after one second without
input.

## Correctness And Determinism

All eight experiments with an output comparison have equal output. Before the
COPY correction, the test compared seeds from 0 to 99. Each file had this
SHA-256 digest:

```text
3f26eb1b456d5a7b71f40a98ab03e0472b05e41cc31e65740dca002da6afa7d6
```

The COPY correction changed the byte format. The baseline format did not
include field lengths. The baseline also put one header and one trailer in each
seed buffer. PostgreSQL rejects that format as one batch.

Version 0.7.0 also corrects two moon values. In version 0.7.0, `sun_distance`
and `inside_ds` use the distance from the moon's parent to the star. The
baseline code used the small moon orbital offset.

PostgreSQL 16 accepted the corrected COPY stream. A ten-seed test inserted 640
stars and 2,446 planets. The test used three seeds per batch. The last batch
contained fewer than three seeds.

Two release tests generated seeds from 0 to 999. The files were byte equal.
Each file contained 50,069,759 bytes and had this SHA-256 digest:

```text
d188c78a555c95f2188db893a8c9890ec2a50848c9b8826f6e72c7104b3b7acd
```

The same release executable wrote seeds from 0 to 499 with 8 workers and with
32 workers. Each run inserted 32,000 stars and 122,266 planets. Sorted database
content had these hashes:

```text
stars:   51ddf9bbe5e3a6dbf17dfa5e65f89621
planets: 32096b2ffd8476930b437c1b9711cb50
```

The result is valid for one compiled executable. A GNU build and a musl build
gave different planet hashes. Galaxy generation uses trigonometric and power
functions. Different math libraries can round these functions differently.
The release uses the pinned musl executable.

## No More Changes

Version 0.7.0 includes the changes marked `Keep` or `Keep for reliability` in
the optimization table. Simplex noise has the largest profile percentage.

The test did not enable unsafe fast math. Fast math can change terrain
comparisons and random-number consumption. The test did not add hand-written
single instruction, multiple data (SIMD) code. SIMD code can also change
floating-point results.

The test did not use profile-guided optimization (PGO). PGO can bind code
layout to one training corpus and one hardware profile.

A dynamic seed queue can reduce range imbalance with fewer workers. A dynamic
seed queue also changes checkpoint rules for seeds that are in progress.
Version 0.7.0 uses more static ranges.

The tests stopped after the recorded experiments. More low-level changes can
change floating-point results or checkpoint rules.
