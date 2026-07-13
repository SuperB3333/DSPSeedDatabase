# Checkpoint Controls

`CHECKPOINT_FREQUENCY` selects persisted checkpoint cadence:

| Value | Write interval |
| --- | ---: |
| `none` | disabled |
| `very_low` | 60 s |
| `low` | 30 s |
| `medium` | 10 s (default) |
| `high` | 1 s |
| `xhigh` | 250 ms |
| `atomic` | 100 ms |

Every enabled write uses a temporary file plus rename, so readers see either the old complete checkpoint or the new complete checkpoint. A final checkpoint is written after worker completion for every enabled frequency.

Checkpoint entries are absolute seed positions for exactly `WORKER_THREADS` workloads. Each entry is rewound by at most 2 seeds, never before its assigned range, to avoid a generation-to-write race losing the newest in-flight entries.

Set `CHECKPOINT_OVERWRITE=1` to remove an existing `CHECKPOINT_FILE` before workload assignment and restart from `START_SEED`. Invalid entry counts and positions are rejected with a message that identifies `CHECKPOINT_OVERWRITE=1` as the explicit recovery action.
