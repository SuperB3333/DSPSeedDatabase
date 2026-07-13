# Runtime Diagnostics

Set `DIAGNOSTICS=1` to print aggregate pipeline measurements at process exit:

- `diagnostics.elapsed_seconds`: wall-clock duration.
- `diagnostics.generated_seeds`, `diagnostics.csv_bytes`, and `diagnostics.csv_mib_per_second`: generated payload volume and wall-clock payload rate.
- `diagnostics.generation_aggregate_seconds` and `diagnostics.generation_average_ms`: CSV-generation cost.
- `diagnostics.channel_send_wait_aggregate_seconds`: worker time blocked sending to the bounded channel.
- `diagnostics.writer_connection_aggregate_seconds`: PostgreSQL connection setup time.
- `diagnostics.batch_receive_aggregate_seconds`, `diagnostics.transaction_start_aggregate_seconds`, `diagnostics.star_copy_aggregate_seconds`, `diagnostics.planet_copy_aggregate_seconds`, and `diagnostics.commit_aggregate_seconds`: writer-stage time.
- `diagnostics.transactions` and `diagnostics.batch_size_min`, `diagnostics.batch_size_average`, and `diagnostics.batch_size_max`: actual, rather than configured, batching behavior.

All stage durations are aggregate thread time. They can exceed `diagnostics.elapsed_seconds` when worker or writer threads overlap. Diagnostics are disabled by default.
