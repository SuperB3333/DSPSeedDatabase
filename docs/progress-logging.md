# Progress Logging

Progress logging is enabled by default and runs in a dedicated sleeping thread. Set `PROGRESS_LOG=0` to disable the logger and all additional stage timers. Set `PROGRESS_LOG_INTERVAL_SECS` to a positive integer number of seconds; the default is `30`.

The logger prints `generation started` before workers launch, then emits time-based reports containing:

- Generated, committed, and overall progress as exact counts and percentages.
- Interval and whole-run generation and commit rates in seeds/second.
- Elapsed time and channel occupancy.
- Aggregate worker and writer-pipeline durations. Writer-pipeline time includes connection, channel receive, COPY, and commit time; use the independent runtime-diagnostics branch when those database stages must be separated. While threads remain active, time is estimated as elapsed time per active thread; final values are measured.
- Active and completed worker/writer counts, dominant aggregate thread-time stage, and a queue-based likely bottleneck.
- Linux process RSS, peak RSS, bytes read, and bytes written from `/proc`; unsupported platforms report `n/a`.

Aggregate stage durations are thread time, so overlapping workers or writers can produce totals larger than wall-clock elapsed time. `likely_bottleneck` is a queue-based diagnostic: at least `80%` occupancy indicates database pressure, at most `20%` indicates generation pressure, and an empty generation phase with outstanding commits indicates database drain.

Instrumentation does not add a timer or branch to each generation pass. Timing occurs once per worker or writer lifetime, and process/resource collection runs only in the monitor thread at the configured interval.
