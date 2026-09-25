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

The Rust report separates uncached config/client construction, a cache hit, a
Scan using a reused client plus JSON serialization, and the `scan_items`
end-to-end path. It prints median, p95, min, and max over 40 samples. The Dart
benchmark is compiled to an AOT executable and reports median/p95 page-conversion
time after warmup. The UI benchmark uses Flutter profile mode and the actual
`DynamoItemsList` widget; its frame-timing report is produced by `integration_test`.

## Initial local baseline (2026-09-24)

Host: CachyOS Linux 7.2.3, AMD Ryzen 9 7900 (12 cores / 24 threads), 61 GiB
RAM; Flutter 3.47.4 / Dart 3.13.3; Rust 1.96.0; DynamoDB Local 2.6.1 (image
`sha256:1856c05cc66a0e49dc1099e483ad2851477eeebe2135250ac11a1d1227db54b1`).
Four independent runs were made for each benchmark.

| Path | Median per run (range) | p95 per run (range) | Fixture |
|---|---:|---:|---|
| Dart `DynamoItem` conversion | 0.636–0.662 ms/page | 0.729–0.777 ms/page | 50 JSON records; decode + pretty-print; AOT |
| Rust uncached client config/build | 0.203–0.236 ms | 0.238–0.414 ms | AWS config/profile resolution + SDK client |
| Reused Rust client `Scan` + JSON | 2.922–3.319 ms/page | 4.974–6.058 ms/page | 50 records, 512-byte payload; warm client |
| Pre-cache Rust `scan_items` path | 4.816–5.015 ms/page | 7.244–10.486 ms/page | Same table/page; constructs a client per call |
| Flutter list scrolling, frame build | 0.824–0.961 ms average | 2.489–2.912 ms p99 | Actual list widget, 50 rows, profile mode |
| Flutter list scrolling, raster | 0.521–0.654 ms average | 0.998–1.318 ms p99 | Same run; 0 missed frame budgets |

The pre-cache local Scan comparison put the `scan_items` pipeline about
1.6–2.1 ms/page slower at the median than the reused-client path. Client
config/build alone was about 0.2 ms; therefore config parsing was not the whole
delta. Connection reuse, request setup, and logging were mixed into that
comparison. This local result does not predict internet/TLS/AWS service latency.

## Client-cache experiment

The working-tree prototype reuses SDK clients by profile, region override, and
endpoint override. It has a five-minute configuration TTL, a 16-entry LRU cap,
single-flight initialization for concurrent requests, and invalidation after an
explicit `aws login` or `aws sso login` succeeds. The SDK's refreshing credential
provider stays attached to each client; the app does not extract or log raw
credentials. Changes to AWS profile files made outside the app are picked up
when the entry expires (within five minutes).

Seven post-cache local runs of the final implementation measured cache hits at
0.1–0.3 µs and `scan_items` at 2.22–3.12 ms median / 5.37–7.06 ms p95. The
median of run medians changed from 4.99 ms to 2.60 ms (~48% lower); the median
p95 changed from 8.85 ms to 6.56 ms (~26% lower). The direct reused-client
comparison varied more than the app-path samples, so treat these as local
directional evidence, not an AWS latency guarantee.

The pinned AWS SDK source wraps its default credential chain in a refreshing
provider and supports refreshing SSO tokens while they remain refreshable.
Successful in-app login invalidates the profile entry; edits made outside the
app are reloaded at the five-minute TTL. This was verified against the SDK's
provider implementation, not a live SSO account; a real AWS latency run remains
outstanding before making production claims.

The 50-row Flutter list and JSON conversion do not currently show a frame or
page-processing bottleneck on this host: every UI run missed zero frame budgets,
and the p99 frame-build time stayed below 3 ms. Larger page sizes, lower-end
hardware, and production AWS remain unmeasured; repeat with those workloads
before generalizing.
