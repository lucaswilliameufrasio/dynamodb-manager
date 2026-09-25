# Performance benchmarks

Run the benchmarks independently so CPU, widget-frame, and DynamoDB-local timings
remain attributable to one layer:

```bash
make perf-dart            # Dart JSON decode + pretty-print for a 50-item page
make perf-dynamodb-local  # Rust SDK config + Scan + AttributeValue-to-JSON
make perf-ui              # Flutter profile-mode frames while scrolling 50 rows
make performance          # Run all three
```

`perf-dynamodb-local` starts a disposable, in-memory DynamoDB Local 2.6.1
container on `127.0.0.1:18000`, uses synthetic credentials, seeds 50 items, and
stops/removes the container afterward. Set `DDB_PERF_PORT` to use another local
port. It requires Docker and `curl`; it never calls an AWS account. `perf-ui`
requires the Flutter Linux desktop target.

The Rust report separates config/client construction, a Scan using a reused
client plus JSON serialization, and the current `scan_items` path (which builds
the client on every call). It prints median, p95, min, and max over 40 samples.
The Dart benchmark is compiled to an AOT executable and reports median/p95
page-conversion time after warmup. The UI
benchmark uses Flutter profile mode and the actual `DynamoItemsList` widget; its
frame-timing report is produced by `integration_test`.

## Initial local baseline (2026-09-24)

Host: CachyOS Linux 7.2.3, AMD Ryzen 9 7900 (12 cores / 24 threads), 61 GiB
RAM; Flutter 3.47.4 / Dart 3.13.3; Rust 1.96.0; DynamoDB Local 2.6.1 (image
`sha256:1856c05cc66a0e49dc1099e483ad2851477eeebe2135250ac11a1d1227db54b1`).
Four independent runs were made for each benchmark.

| Path | Median per run (range) | p95 per run (range) | Fixture |
|---|---:|---:|---|
| Dart `DynamoItem` conversion | 0.636–0.662 ms/page | 0.729–0.777 ms/page | 50 JSON records; decode + pretty-print; AOT |
| Rust client config/build only | 0.203–0.236 ms | 0.238–0.414 ms | AWS config/profile resolution + SDK client |
| Reused Rust client `Scan` + JSON | 2.922–3.319 ms/page | 4.974–6.058 ms/page | 50 records, 512-byte payload; warm client |
| Current Rust `scan_items` path | 4.816–5.015 ms/page | 7.244–10.486 ms/page | Same table/page; constructs a client per call |
| Flutter list scrolling, frame build | 0.824–0.961 ms average | 2.489–2.912 ms p99 | Actual list widget, 50 rows, profile mode |
| Flutter list scrolling, raster | 0.521–0.654 ms average | 0.998–1.318 ms p99 | Same run; 0 missed frame budgets |

The local Scan comparison puts the current pipeline about 1.6–2.1 ms/page
slower at the median than the reused-client path. Client config/build alone is
about 0.2 ms; therefore config parsing is not the whole delta. Connection reuse,
request setup, and logging remain mixed into this comparison and need targeted
profiling before changing client lifetime. This local result does not predict
internet/TLS/AWS service latency.

The 50-row Flutter list and JSON conversion do not currently show a frame or
page-processing bottleneck on this host: every UI run missed zero frame budgets,
and the p99 frame-build time stayed below 3 ms. Larger page sizes, lower-end
hardware, and production AWS remain unmeasured; repeat with those workloads
before generalizing.
