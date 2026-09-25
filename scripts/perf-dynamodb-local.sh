#!/usr/bin/env bash
set -euo pipefail

container_name="dynamodb-manager-perf-${RANDOM}-$$"
port="${DDB_PERF_PORT:-18000}"
endpoint="http://127.0.0.1:${port}"

container_id="$(docker run --rm --detach \
  --name "$container_name" \
  --publish "127.0.0.1:${port}:8000" \
  amazon/dynamodb-local:2.6.1 \
  -jar DynamoDBLocal.jar -sharedDb -inMemory)"

cleanup() {
  docker stop "$container_id" >/dev/null 2>&1 || true
}
trap cleanup EXIT

ready=false
for _ in $(seq 1 60); do
  status="$(curl --silent --output /dev/null --write-out '%{http_code}' \
    --request POST \
    --header 'content-type: application/x-amz-json-1.0' \
    --header 'x-amz-target: DynamoDB_20120810.ListTables' \
    --data '{}' "$endpoint" || true)"
  if [[ "$status" != "000" ]]; then
    ready=true
    break
  fi
  sleep 0.5
done

if [[ "$ready" != true ]]; then
  docker logs "$container_id" >&2 || true
  echo "DynamoDB Local did not become ready at $endpoint" >&2
  exit 1
fi

echo "Running performance test against disposable DynamoDB Local 2.6.1 at $endpoint"
AWS_ACCESS_KEY_ID=AKIDEXAMPLE0000000001 \
AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY \
AWS_EC2_METADATA_DISABLED=true \
AWS_DEFAULT_REGION=us-east-1 \
DDB_PERF_ENDPOINT="$endpoint" \
cargo test --manifest-path rust/Cargo.toml --release \
  benchmark_scan_page_pipeline_against_local_dynamodb -- --ignored --nocapture
