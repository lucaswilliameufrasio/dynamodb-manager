use crate::api::dev_logs::{log_error, log_info};
use aws_config::{BehaviorVersion, Region};
use aws_sdk_dynamodb::types::{
    AttributeDefinition, AttributeValue, BillingMode, GlobalSecondaryIndex, KeySchemaElement,
    KeyType, Projection, ProjectionType, ProvisionedThroughput, ScalarAttributeType,
};
use aws_sdk_dynamodb::Client as DdbClient;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Duration;

// ─── Data structures exposed to Flutter via FRB ────────────────────────────

#[derive(Debug, Clone)]
pub struct TableSummary {
    pub name: String,
    pub status: String,
    pub arn: Option<String>,
    pub item_count: Option<i64>,
    pub table_size_bytes: Option<i64>,
    pub billing_mode: Option<String>,
    pub pk: Option<String>,
    pub sk: Option<String>,
    pub gsis: Vec<String>,
    pub lsis: Vec<String>,
    pub stream_spec: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ItemsPageResult {
    pub items_json: Vec<String>,
    pub last_evaluated_key_json: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FilterClause {
    pub attribute: String,
    pub condition: String,
    pub value_type: String,
    pub value: Option<String>,
    pub value2: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AttributeHint {
    pub name: String,
    pub types: Vec<String>,
}

struct FilterExpressionParts {
    expression: Option<String>,
    names: Vec<(String, String)>,
    values: Vec<(String, AttributeValue)>,
}

const CONFIG_TIMEOUT: Duration = Duration::from_secs(8);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Run a future with a timeout and produce a user-facing error if it elapses.
async fn with_timeout<T, F: std::future::Future<Output = Result<T, String>>>(
    fut: F,
    profile: &str,
    label: &str,
    timeout: Duration,
) -> Result<T, String> {
    match tokio::time::timeout(timeout, fut).await {
        Ok(result) => result,
        Err(_) => Err(format!(
            "Timed out after {}s while {} for profile '{}'",
            timeout.as_secs(),
            label,
            profile
        )),
    }
}

// ─── Client builder ────────────────────────────────────────────────────────

async fn build_ddb_client(
    profile: &str,
    region_override: Option<String>,
    endpoint_override: Option<String>,
) -> Result<DdbClient, String> {
    log_info(
        "dynamodb",
        format!("build_ddb_client start profile='{}'", profile),
    );
    let mut loader = aws_config::defaults(BehaviorVersion::latest()).profile_name(profile);
    if let Some(region) = region_override {
        loader = loader.region(Region::new(region));
    }
    if let Some(ep) = endpoint_override {
        loader = loader.endpoint_url(ep);
    }

    let config = with_timeout(
        async move {
            let cfg = loader.load().await;
            log_info("dynamodb", format!("config loaded profile='{}'", profile));
            Ok(cfg)
        },
        profile,
        "resolving AWS credentials and config",
        CONFIG_TIMEOUT,
    )
    .await
    .map_err(|e| {
        log_error(
            "dynamodb",
            format!("config timeout profile='{}': {}", profile, e),
        );
        e
    })?;

    Ok(DdbClient::new(&config))
}

// ─── Table operations ──────────────────────────────────────────────────────

pub async fn list_tables(
    profile: String,
    region_override: Option<String>,
    endpoint_override: Option<String>,
) -> Result<Vec<String>, String> {
    log_info(
        "dynamodb",
        format!("list_tables start profile='{}'", profile),
    );
    let client = build_ddb_client(&profile, region_override, endpoint_override).await?;
    let mut tables = Vec::new();
    let mut start = None;

    loop {
        let mut req = client.list_tables().limit(100);
        if let Some(s) = start {
            req = req.exclusive_start_table_name(s);
        }

        let result = with_timeout(
            async { req.send().await.map_err(|e| e.to_string()) },
            &profile,
            "listing DynamoDB tables",
            REQUEST_TIMEOUT,
        )
        .await?;

        tables.extend(result.table_names.unwrap_or_default());
        start = result.last_evaluated_table_name;
        if start.is_none() {
            break;
        }
    }

    tables.sort();
    log_info(
        "dynamodb",
        format!(
            "list_tables done count={} profile='{}'",
            tables.len(),
            profile
        ),
    );
    Ok(tables)
}

/// Create a DynamoDB table with an optional sort key and global secondary indexes.
#[allow(clippy::too_many_arguments)]
pub async fn create_table(
    profile: String,
    region_override: Option<String>,
    endpoint_override: Option<String>,
    table_name: String,
    pk_name: String,
    pk_type: String,
    sk_name: Option<String>,
    sk_type: Option<String>,
    billing_mode: String,
    read_capacity: Option<i64>,
    write_capacity: Option<i64>,
    gsis_json: String,
) -> Result<(), String> {
    log_info(
        "dynamodb",
        format!(
            "create_table start table='{}' profile='{}'",
            table_name, profile
        ),
    );
    if table_name.trim().is_empty() || pk_name.trim().is_empty() {
        return Err("Table name and partition key name are required".to_string());
    }
    if !matches!(billing_mode.as_str(), "PROVISIONED" | "PAY_PER_REQUEST") {
        return Err("Billing mode must be PROVISIONED or PAY_PER_REQUEST".to_string());
    }
    if billing_mode == "PROVISIONED"
        && (read_capacity.unwrap_or(5) < 1 || write_capacity.unwrap_or(5) < 1)
    {
        return Err("Provisioned read and write capacity must be at least 1".to_string());
    }
    if sk_name
        .as_deref()
        .is_some_and(|sk| sk.trim() == pk_name.trim())
    {
        return Err("Partition and sort keys must use different attribute names".to_string());
    }

    let mut attributes = BTreeMap::new();
    attributes.insert(pk_name.clone(), parse_scalar_attribute_type(&pk_type)?);
    let mut key_schema = vec![KeySchemaElement::builder()
        .attribute_name(&pk_name)
        .key_type(KeyType::Hash)
        .build()
        .map_err(|e| e.to_string())?];

    if let Some(sk) = sk_name.as_ref().filter(|name| !name.trim().is_empty()) {
        let ty = sk_type.as_deref().unwrap_or("S");
        if let Some(existing) = attributes.get(sk) {
            if existing.as_str() != ty {
                return Err(format!(
                    "Attribute '{sk}' is used with conflicting key types"
                ));
            }
        }
        attributes.insert(sk.clone(), parse_scalar_attribute_type(ty)?);
        key_schema.push(
            KeySchemaElement::builder()
                .attribute_name(sk)
                .key_type(KeyType::Range)
                .build()
                .map_err(|e| e.to_string())?,
        );
    }

    let gsi_specs: Vec<serde_json::Value> =
        serde_json::from_str(&gsis_json).map_err(|e| format!("Invalid GSI configuration: {e}"))?;
    if gsi_specs.len() > 20 {
        return Err("A table can have at most 20 global secondary indexes".to_string());
    }
    let mut gsis = Vec::new();
    let mut index_names = BTreeSet::new();
    for gsi in gsi_specs {
        let index_name = gsi["name"].as_str().unwrap_or_default().trim();
        let index_pk = gsi["pk_name"].as_str().unwrap_or_default().trim();
        if index_name.is_empty() || index_pk.is_empty() {
            return Err("Each GSI requires an index name and partition key".to_string());
        }
        if !index_names.insert(index_name.to_string()) {
            return Err(format!(
                "Duplicate global secondary index name '{index_name}'"
            ));
        }
        for (name, ty) in [
            (index_pk, gsi["pk_type"].as_str().unwrap_or("S")),
            (
                gsi["sk_name"].as_str().unwrap_or_default().trim(),
                gsi["sk_type"].as_str().unwrap_or("S"),
            ),
        ] {
            if !name.is_empty() {
                let parsed = parse_scalar_attribute_type(ty)?;
                if attributes.get(name).is_some_and(|old| old.as_str() != ty) {
                    return Err(format!(
                        "Attribute '{name}' is used with conflicting key types"
                    ));
                }
                attributes.insert(name.to_string(), parsed);
            }
        }
        let mut schema = vec![KeySchemaElement::builder()
            .attribute_name(index_pk)
            .key_type(KeyType::Hash)
            .build()
            .map_err(|e| e.to_string())?];
        if let Some(index_sk) = gsi["sk_name"]
            .as_str()
            .filter(|name| !name.trim().is_empty())
        {
            if index_sk.trim() == index_pk {
                return Err(format!(
                    "GSI '{index_name}' partition and sort keys must be different"
                ));
            }
            schema.push(
                KeySchemaElement::builder()
                    .attribute_name(index_sk)
                    .key_type(KeyType::Range)
                    .build()
                    .map_err(|e| e.to_string())?,
            );
        }
        let mut index = GlobalSecondaryIndex::builder()
            .index_name(index_name)
            .set_key_schema(Some(schema))
            .projection(
                Projection::builder()
                    .projection_type(ProjectionType::All)
                    .build(),
            );
        if billing_mode == "PROVISIONED" {
            index = index.provisioned_throughput(
                ProvisionedThroughput::builder()
                    .read_capacity_units(read_capacity.unwrap_or(5))
                    .write_capacity_units(write_capacity.unwrap_or(5))
                    .build()
                    .map_err(|e| e.to_string())?,
            );
        }
        gsis.push(index.build().map_err(|e| e.to_string())?);
    }

    let attribute_definitions = attributes
        .into_iter()
        .map(|(name, ty)| {
            AttributeDefinition::builder()
                .attribute_name(name)
                .attribute_type(ty)
                .build()
                .map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    let client = build_ddb_client(&profile, region_override, endpoint_override).await?;
    let mut request = client
        .create_table()
        .table_name(&table_name)
        .set_attribute_definitions(Some(attribute_definitions))
        .set_key_schema(Some(key_schema))
        .billing_mode(match billing_mode.as_str() {
            "PROVISIONED" => BillingMode::Provisioned,
            _ => BillingMode::PayPerRequest,
        })
        .set_global_secondary_indexes(if gsis.is_empty() { None } else { Some(gsis) });

    if billing_mode == "PROVISIONED" {
        request = request.provisioned_throughput(
            ProvisionedThroughput::builder()
                .read_capacity_units(read_capacity.unwrap_or(5))
                .write_capacity_units(write_capacity.unwrap_or(5))
                .build()
                .map_err(|e| e.to_string())?,
        );
    }

    with_timeout(
        async { request.send().await.map(|_| ()).map_err(|e| e.to_string()) },
        &profile,
        &format!("creating table '{table_name}'"),
        REQUEST_TIMEOUT,
    )
    .await?;
    log_info(
        "dynamodb",
        format!("create_table accepted table='{}'", table_name),
    );
    Ok(())
}

fn parse_scalar_attribute_type(value: &str) -> Result<ScalarAttributeType, String> {
    match value {
        "S" => Ok(ScalarAttributeType::S),
        "N" => Ok(ScalarAttributeType::N),
        "B" => Ok(ScalarAttributeType::B),
        _ => Err(format!("Unsupported key type '{value}'. Use S, N or B.")),
    }
}

pub async fn delete_table(
    profile: String,
    region_override: Option<String>,
    endpoint_override: Option<String>,
    table_name: String,
) -> Result<(), String> {
    log_info(
        "dynamodb",
        format!(
            "delete_table start table='{}' profile='{}'",
            table_name, profile
        ),
    );
    if table_name.trim().is_empty() {
        return Err("Table name is required".to_string());
    }
    let client = build_ddb_client(&profile, region_override, endpoint_override).await?;
    with_timeout(
        async {
            client
                .delete_table()
                .table_name(&table_name)
                .send()
                .await
                .map(|_| ())
                .map_err(|e| e.to_string())
        },
        &profile,
        &format!("deleting table '{table_name}'"),
        REQUEST_TIMEOUT,
    )
    .await?;
    log_info(
        "dynamodb",
        format!("delete_table accepted table='{}'", table_name),
    );
    Ok(())
}

#[cfg(test)]
mod table_operation_tests {
    use super::*;

    #[test]
    fn parses_supported_scalar_types() {
        assert_eq!(
            parse_scalar_attribute_type("S").unwrap(),
            ScalarAttributeType::S
        );
        assert_eq!(
            parse_scalar_attribute_type("N").unwrap(),
            ScalarAttributeType::N
        );
        assert_eq!(
            parse_scalar_attribute_type("B").unwrap(),
            ScalarAttributeType::B
        );
        assert!(parse_scalar_attribute_type("BOOL").is_err());
    }

    #[tokio::test]
    async fn create_table_rejects_invalid_input_before_aws_access() {
        let result = create_table(
            String::new(),
            None,
            None,
            "table".to_string(),
            "pk".to_string(),
            "invalid".to_string(),
            None,
            None,
            "PAY_PER_REQUEST".to_string(),
            None,
            None,
            "[]".to_string(),
        )
        .await;
        assert!(result.unwrap_err().contains("Unsupported key type"));
    }

    #[tokio::test]
    async fn create_table_rejects_invalid_billing_and_capacity() {
        let invalid_billing = create_table(
            String::new(),
            None,
            None,
            "table".to_string(),
            "pk".to_string(),
            "S".to_string(),
            None,
            None,
            "UNKNOWN".to_string(),
            None,
            None,
            "[]".to_string(),
        )
        .await;
        assert!(invalid_billing.unwrap_err().contains("Billing mode"));

        let invalid_capacity = create_table(
            String::new(),
            None,
            None,
            "table".to_string(),
            "pk".to_string(),
            "S".to_string(),
            None,
            None,
            "PROVISIONED".to_string(),
            Some(0),
            Some(5),
            "[]".to_string(),
        )
        .await;
        assert!(invalid_capacity.unwrap_err().contains("at least 1"));
    }

    #[tokio::test]
    async fn create_table_rejects_malformed_gsi_before_aws_access() {
        let result = create_table(
            String::new(),
            None,
            None,
            "table".to_string(),
            "pk".to_string(),
            "S".to_string(),
            None,
            None,
            "PAY_PER_REQUEST".to_string(),
            None,
            None,
            r#"[{"name":"by_status"}]"#.to_string(),
        )
        .await;
        assert!(result
            .unwrap_err()
            .contains("requires an index name and partition key"));
    }

    #[tokio::test]
    async fn delete_table_rejects_blank_name_before_aws_access() {
        let result = delete_table(String::new(), None, None, "  ".to_string()).await;
        assert!(result.unwrap_err().contains("Table name is required"));
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use aws_sdk_dynamodb::types::{PutRequest, WriteRequest};
    use std::hint::black_box;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    const PAGE_SIZE: i32 = 50;
    const SAMPLE_COUNT: usize = 40;

    #[tokio::test]
    #[ignore = "run via make perf-dynamodb-local; requires disposable DynamoDB Local"]
    async fn benchmark_scan_page_pipeline_against_local_dynamodb() {
        let endpoint = std::env::var("DDB_PERF_ENDPOINT")
            .unwrap_or_else(|_| "http://127.0.0.1:18000".to_string());
        let profile = "default".to_string();
        let region = Some("us-east-1".to_string());
        let table_name = format!(
            "dynamodb_manager_perf_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        let client = build_ddb_client(&profile, region.clone(), Some(endpoint.clone()))
            .await
            .expect("build local benchmark client");
        let key_attribute = AttributeDefinition::builder()
            .attribute_name("pk")
            .attribute_type(ScalarAttributeType::S)
            .build()
            .unwrap();
        let partition_key = KeySchemaElement::builder()
            .attribute_name("pk")
            .key_type(KeyType::Hash)
            .build()
            .unwrap();
        client
            .create_table()
            .table_name(&table_name)
            .attribute_definitions(key_attribute)
            .key_schema(partition_key)
            .billing_mode(BillingMode::PayPerRequest)
            .send()
            .await
            .unwrap_or_else(|error| panic!("create local benchmark table: {error:?}"));

        let mut active = false;
        for _ in 0..100 {
            if let Ok(description) = client.describe_table().table_name(&table_name).send().await {
                if description
                    .table
                    .and_then(|table| {
                        table
                            .table_status()
                            .map(|status| status.as_str().to_string())
                    })
                    .as_deref()
                    == Some("ACTIVE")
                {
                    active = true;
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(active, "local benchmark table did not become ACTIVE");

        for batch_start in (0..PAGE_SIZE).step_by(25) {
            let requests = (batch_start..(batch_start + 25).min(PAGE_SIZE))
                .map(|index| {
                    let item = HashMap::from([
                        (
                            "pk".to_string(),
                            AttributeValue::S(format!("item-{index:03}")),
                        ),
                        (
                            "status".to_string(),
                            AttributeValue::S(if index % 2 == 0 {
                                "complete".to_string()
                            } else {
                                "pending".to_string()
                            }),
                        ),
                        (
                            "amount".to_string(),
                            AttributeValue::N((index as f64 * 12.75).to_string()),
                        ),
                        ("payload".to_string(), AttributeValue::S("x".repeat(512))),
                    ]);
                    let put = PutRequest::builder().set_item(Some(item)).build().unwrap();
                    WriteRequest::builder().put_request(put).build()
                })
                .collect::<Vec<_>>();
            client
                .batch_write_item()
                .request_items(&table_name, requests)
                .send()
                .await
                .expect("seed local benchmark table");
        }

        // Warm the JIT, local HTTP connection, and both SDK connection pools.
        for _ in 0..5 {
            let (_, reused_count) = scan_page_with_reused_client(&client, &table_name)
                .await
                .expect("warm-up reused-client scan");
            assert_eq!(reused_count, PAGE_SIZE as usize);
            let page = scan_items(
                profile.clone(),
                region.clone(),
                Some(endpoint.clone()),
                table_name.clone(),
                Some(PAGE_SIZE),
                None,
                None,
            )
            .await
            .expect("warm-up scan");
            assert_eq!(page.items_json.len(), PAGE_SIZE as usize);
        }

        let mut client_build_samples = Vec::with_capacity(SAMPLE_COUNT);
        for _ in 0..SAMPLE_COUNT {
            let started = Instant::now();
            let built = build_ddb_client(&profile, region.clone(), Some(endpoint.clone()))
                .await
                .expect("build benchmark client");
            client_build_samples.push(started.elapsed());
            drop(built);
        }

        let mut reused_client_samples = Vec::with_capacity(SAMPLE_COUNT);
        let mut app_pipeline_samples = Vec::with_capacity(SAMPLE_COUNT);
        let mut total_items = 0usize;
        for sample in 0..SAMPLE_COUNT {
            if sample % 2 == 0 {
                let (elapsed, count) = scan_page_with_reused_client(&client, &table_name)
                    .await
                    .expect("reused-client scan");
                reused_client_samples.push(elapsed);
                total_items += count;

                let (elapsed, count) = scan_page_with_application_api(
                    &profile,
                    region.clone(),
                    &endpoint,
                    &table_name,
                )
                .await
                .expect("application scan pipeline");
                app_pipeline_samples.push(elapsed);
                total_items += count;
            } else {
                let (elapsed, count) = scan_page_with_application_api(
                    &profile,
                    region.clone(),
                    &endpoint,
                    &table_name,
                )
                .await
                .expect("application scan pipeline");
                app_pipeline_samples.push(elapsed);
                total_items += count;

                let (elapsed, count) = scan_page_with_reused_client(&client, &table_name)
                    .await
                    .expect("reused-client scan");
                reused_client_samples.push(elapsed);
                total_items += count;
            }
        }

        print_distribution("client_config_only", &client_build_samples);
        print_distribution("reused_client_scan_and_json", &reused_client_samples);
        print_distribution("app_scan_items_end_to_end", &app_pipeline_samples);
        println!(
            "fixture=items:{PAGE_SIZE},payload_bytes:512,samples:{SAMPLE_COUNT},endpoint:dynamodb-local"
        );
        println!("checksum_items={total_items}");

        client
            .delete_table()
            .table_name(&table_name)
            .send()
            .await
            .expect("delete local benchmark table");
    }

    async fn scan_page_with_reused_client(
        client: &DdbClient,
        table_name: &str,
    ) -> Result<(Duration, usize), String> {
        let started = Instant::now();
        let response = client
            .scan()
            .table_name(table_name)
            .limit(PAGE_SIZE)
            .send()
            .await
            .map_err(|error| format!("{error:?}"))?;
        let serialized = response
            .items
            .unwrap_or_default()
            .iter()
            .map(attr_map_to_json_string)
            .collect::<Result<Vec<_>, _>>()?;
        let item_count = serialized.len();
        black_box(serialized);
        Ok((started.elapsed(), item_count))
    }

    async fn scan_page_with_application_api(
        profile: &str,
        region: Option<String>,
        endpoint: &str,
        table_name: &str,
    ) -> Result<(Duration, usize), String> {
        let started = Instant::now();
        let page = scan_items(
            profile.to_string(),
            region,
            Some(endpoint.to_string()),
            table_name.to_string(),
            Some(PAGE_SIZE),
            None,
            None,
        )
        .await?;
        let item_count = page.items_json.len();
        black_box(page.items_json);
        Ok((started.elapsed(), item_count))
    }

    fn print_distribution(label: &str, samples: &[Duration]) {
        let mut micros = samples
            .iter()
            .map(|sample| sample.as_secs_f64() * 1_000_000.0)
            .collect::<Vec<_>>();
        micros.sort_by(f64::total_cmp);
        let median = percentile(&micros, 0.50);
        let p95 = percentile(&micros, 0.95);
        println!(
            "{label}_us median={median:.1} p95={p95:.1} min={:.1} max={:.1}",
            micros[0],
            micros[micros.len() - 1]
        );
    }

    fn percentile(sorted: &[f64], fraction: f64) -> f64 {
        let index = ((sorted.len() - 1) as f64 * fraction).ceil() as usize;
        sorted[index]
    }
}

pub async fn describe_table(
    profile: String,
    region_override: Option<String>,
    endpoint_override: Option<String>,
    table_name: String,
) -> Result<TableSummary, String> {
    log_info(
        "dynamodb",
        format!(
            "describe_table start table='{}' profile='{}'",
            table_name, profile
        ),
    );
    let client = build_ddb_client(&profile, region_override, endpoint_override).await?;

    let response = with_timeout(
        async {
            client
                .describe_table()
                .table_name(&table_name)
                .send()
                .await
                .map_err(|e| e.to_string())
        },
        &profile,
        &format!("describing table '{}'", table_name),
        REQUEST_TIMEOUT,
    )
    .await?;

    let table = response.table.ok_or("No table found")?;

    let status = table
        .table_status()
        .map(|s| s.as_str().to_string())
        .unwrap_or_else(|| "?".to_string());

    let arn = table.table_arn().map(|s| s.to_string());
    let item_count = table.item_count();
    let table_size_bytes = table.table_size_bytes();

    let billing_mode = table
        .billing_mode_summary()
        .and_then(|b| b.billing_mode())
        .map(|m| m.as_str().to_string());

    let mut pk: Option<String> = None;
    let mut sk: Option<String> = None;
    for ks in table.key_schema().iter() {
        let attr = ks.attribute_name().to_string();
        match ks.key_type() {
            aws_sdk_dynamodb::types::KeyType::Hash => pk = Some(attr),
            aws_sdk_dynamodb::types::KeyType::Range => sk = Some(attr),
            _ => {}
        }
    }

    let gsis = table
        .global_secondary_indexes()
        .iter()
        .filter_map(|g| g.index_name().map(|s| s.to_string()))
        .collect();
    let lsis = table
        .local_secondary_indexes()
        .iter()
        .filter_map(|g| g.index_name().map(|s| s.to_string()))
        .collect();

    let stream_spec = table.stream_specification().map(|s| {
        if s.stream_enabled() {
            let view = s
                .stream_view_type()
                .map(|v| v.as_str().to_string())
                .unwrap_or_default();
            format!("enabled {}", view)
        } else {
            "disabled".to_string()
        }
    });

    Ok(TableSummary {
        name: table_name,
        status,
        arn,
        item_count,
        table_size_bytes,
        billing_mode,
        pk,
        sk,
        gsis,
        lsis,
        stream_spec,
    })
}

// ─── Item scan / query ─────────────────────────────────────────────────────

pub async fn scan_items(
    profile: String,
    region_override: Option<String>,
    endpoint_override: Option<String>,
    table_name: String,
    limit: Option<i32>,
    exclusive_start_key_json: Option<String>,
    filters: Option<Vec<FilterClause>>,
) -> Result<ItemsPageResult, String> {
    log_info(
        "dynamodb",
        format!(
            "scan_items start table='{}' profile='{}'",
            table_name, profile
        ),
    );
    let client = build_ddb_client(&profile, region_override, endpoint_override).await?;
    let mut request = client.scan().table_name(&table_name);

    if let Some(l) = limit {
        request = request.limit(l);
    }
    if let Some(json) = exclusive_start_key_json {
        let attr_map = json_str_to_attr_map(&json)?;
        request = request.set_exclusive_start_key(Some(attr_map));
    }
    if let Some(f) = filters {
        let parts = build_filter_expression_parts(&f)?;
        if let Some(expr) = parts.expression {
            request = request.filter_expression(expr);
        }
        for (k, v) in parts.names {
            request = request.expression_attribute_names(k, v);
        }
        for (k, v) in parts.values {
            request = request.expression_attribute_values(k, v);
        }
    }

    let response = with_timeout(
        async { request.send().await.map_err(|e| e.to_string()) },
        &profile,
        &format!("scanning table '{}'", table_name),
        REQUEST_TIMEOUT,
    )
    .await?;

    let items_json: Vec<String> = response
        .items
        .unwrap_or_default()
        .into_iter()
        .map(|item| attr_map_to_json_string(&item).unwrap_or_else(|_| "null".to_string()))
        .collect();

    let last_evaluated_key_json = response
        .last_evaluated_key
        .and_then(|m| attr_map_to_json_string(&m).ok());

    log_info(
        "dynamodb",
        format!(
            "scan_items done count={} table='{}'",
            items_json.len(),
            table_name
        ),
    );

    Ok(ItemsPageResult {
        items_json,
        last_evaluated_key_json,
    })
}

/// Query items with partition-key filter, pagination and optional filter expressions.
#[allow(clippy::too_many_arguments)]
pub async fn query_items(
    profile: String,
    region_override: Option<String>,
    endpoint_override: Option<String>,
    table_name: String,
    pk_name: String,
    pk_value: String,
    limit: Option<i32>,
    exclusive_start_key_json: Option<String>,
    filters: Option<Vec<FilterClause>>,
) -> Result<ItemsPageResult, String> {
    log_info(
        "dynamodb",
        format!(
            "query_items start table='{}' profile='{}'",
            table_name, profile
        ),
    );
    let client = build_ddb_client(&profile, region_override, endpoint_override).await?;
    let mut request = client
        .query()
        .table_name(&table_name)
        .key_condition_expression("#pk = :pkv")
        .expression_attribute_names("#pk", &pk_name)
        .expression_attribute_values(":pkv", AttributeValue::S(pk_value));

    if let Some(l) = limit {
        request = request.limit(l);
    }
    if let Some(json) = exclusive_start_key_json {
        let attr_map = json_str_to_attr_map(&json)?;
        request = request.set_exclusive_start_key(Some(attr_map));
    }
    if let Some(f) = filters {
        let parts = build_filter_expression_parts(&f)?;
        if let Some(expr) = parts.expression {
            request = request.filter_expression(expr);
        }
        for (k, v) in parts.names {
            request = request.expression_attribute_names(k, v);
        }
        for (k, v) in parts.values {
            request = request.expression_attribute_values(k, v);
        }
    }

    let response = with_timeout(
        async { request.send().await.map_err(|e| e.to_string()) },
        &profile,
        &format!("querying table '{}'", table_name),
        REQUEST_TIMEOUT,
    )
    .await?;

    let items_json: Vec<String> = response
        .items
        .unwrap_or_default()
        .into_iter()
        .map(|item| attr_map_to_json_string(&item).unwrap_or_else(|_| "null".to_string()))
        .collect();

    let last_evaluated_key_json = response
        .last_evaluated_key
        .and_then(|m| attr_map_to_json_string(&m).ok());

    log_info(
        "dynamodb",
        format!(
            "query_items done count={} table='{}'",
            items_json.len(),
            table_name
        ),
    );

    Ok(ItemsPageResult {
        items_json,
        last_evaluated_key_json,
    })
}

// ─── Item mutations ────────────────────────────────────────────────────────

pub async fn put_item(
    profile: String,
    region_override: Option<String>,
    endpoint_override: Option<String>,
    table_name: String,
    item_json: String,
) -> Result<(), String> {
    log_info(
        "dynamodb",
        format!(
            "put_item start table='{}' profile='{}'",
            table_name, profile
        ),
    );
    let client = build_ddb_client(&profile, region_override, endpoint_override).await?;
    let map = json_str_to_attr_map(&item_json)?;

    with_timeout(
        async {
            client
                .put_item()
                .table_name(&table_name)
                .set_item(Some(map))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        },
        &profile,
        &format!("putting item in table '{}'", table_name),
        REQUEST_TIMEOUT,
    )
    .await
}

pub async fn put_item_create_only(
    profile: String,
    region_override: Option<String>,
    endpoint_override: Option<String>,
    table_name: String,
    item_json: String,
    pk_name: String,
    sk_name: Option<String>,
) -> Result<(), String> {
    log_info(
        "dynamodb",
        format!("put_item_create_only start table='{}'", table_name),
    );
    let client = build_ddb_client(&profile, region_override, endpoint_override).await?;
    let map = json_str_to_attr_map(&item_json)?;

    with_timeout(
        async {
            let mut request = client
                .put_item()
                .table_name(&table_name)
                .set_item(Some(map))
                .condition_expression(if sk_name.is_some() {
                    "attribute_not_exists(#pk) AND attribute_not_exists(#sk)"
                } else {
                    "attribute_not_exists(#pk)"
                })
                .expression_attribute_names("#pk", &pk_name);

            if let Some(sk) = sk_name {
                request = request.expression_attribute_names("#sk", sk);
            }

            request.send().await.map_err(|e| e.to_string())?;
            Ok(())
        },
        &profile,
        &format!("create-only put in table '{}'", table_name),
        REQUEST_TIMEOUT,
    )
    .await
}

pub async fn put_item_update_only(
    profile: String,
    region_override: Option<String>,
    endpoint_override: Option<String>,
    table_name: String,
    item_json: String,
    pk_name: String,
    sk_name: Option<String>,
) -> Result<(), String> {
    log_info(
        "dynamodb",
        format!("put_item_update_only start table='{}'", table_name),
    );
    let client = build_ddb_client(&profile, region_override, endpoint_override).await?;
    let map = json_str_to_attr_map(&item_json)?;

    with_timeout(
        async {
            let mut request = client
                .put_item()
                .table_name(&table_name)
                .set_item(Some(map))
                .condition_expression(if sk_name.is_some() {
                    "attribute_exists(#pk) AND attribute_exists(#sk)"
                } else {
                    "attribute_exists(#pk)"
                })
                .expression_attribute_names("#pk", &pk_name);

            if let Some(sk) = sk_name {
                request = request.expression_attribute_names("#sk", sk);
            }

            request.send().await.map_err(|e| e.to_string())?;
            Ok(())
        },
        &profile,
        &format!("update-only put in table '{}'", table_name),
        REQUEST_TIMEOUT,
    )
    .await
}

pub async fn delete_item(
    profile: String,
    region_override: Option<String>,
    endpoint_override: Option<String>,
    table_name: String,
    key_json: String,
) -> Result<(), String> {
    log_info(
        "dynamodb",
        format!(
            "delete_item start table='{}' profile='{}'",
            table_name, profile
        ),
    );
    let client = build_ddb_client(&profile, region_override, endpoint_override).await?;
    let key_map = json_str_to_attr_map(&key_json)?;

    with_timeout(
        async {
            client
                .delete_item()
                .table_name(&table_name)
                .set_key(Some(key_map))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        },
        &profile,
        &format!("deleting item from table '{}'", table_name),
        REQUEST_TIMEOUT,
    )
    .await
}

// ─── Attribute discovery ───────────────────────────────────────────────────

pub async fn list_table_attributes(
    profile: String,
    region_override: Option<String>,
    endpoint_override: Option<String>,
    table_name: String,
    sample_limit: Option<i32>,
) -> Result<Vec<AttributeHint>, String> {
    log_info(
        "dynamodb",
        format!("list_table_attributes start table='{}'", table_name),
    );
    let client = build_ddb_client(&profile, region_override, endpoint_override).await?;

    let response = with_timeout(
        async {
            client
                .scan()
                .table_name(&table_name)
                .limit(sample_limit.unwrap_or(100))
                .send()
                .await
                .map_err(|e| e.to_string())
        },
        &profile,
        &format!("sampling attributes from table '{}'", table_name),
        REQUEST_TIMEOUT,
    )
    .await?;

    let mut by_name: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for item in response.items.unwrap_or_default() {
        for (name, av) in item {
            by_name
                .entry(name)
                .or_default()
                .insert(attr_value_kind(&av).to_string());
        }
    }

    let out = by_name
        .into_iter()
        .map(|(name, types)| AttributeHint {
            name,
            types: types.into_iter().collect(),
        })
        .collect();

    Ok(out)
}

// ─── JSON <-> AttributeValue conversion ────────────────────────────────────

fn json_str_to_attr_map(json: &str) -> Result<HashMap<String, AttributeValue>, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("Invalid JSON: {}", e))?;
    match v {
        serde_json::Value::Object(map) => {
            let mut out = HashMap::new();
            for (k, val) in map {
                out.insert(k, json_val_to_attr_value(&val)?);
            }
            Ok(out)
        }
        _ => Err("Item must be a JSON object".to_string()),
    }
}

fn json_val_to_attr_value(v: &serde_json::Value) -> Result<AttributeValue, String> {
    match v {
        serde_json::Value::String(s) => Ok(AttributeValue::S(s.clone())),
        serde_json::Value::Number(n) => Ok(AttributeValue::N(n.to_string())),
        serde_json::Value::Bool(b) => Ok(AttributeValue::Bool(*b)),
        serde_json::Value::Null => Ok(AttributeValue::Null(true)),
        serde_json::Value::Array(arr) => {
            let mut list = Vec::with_capacity(arr.len());
            for el in arr {
                list.push(json_val_to_attr_value(el)?);
            }
            Ok(AttributeValue::L(list))
        }
        serde_json::Value::Object(map) => {
            let mut m = HashMap::new();
            for (k, val) in map {
                m.insert(k.clone(), json_val_to_attr_value(val)?);
            }
            Ok(AttributeValue::M(m))
        }
    }
}

fn attr_map_to_json_string(map: &HashMap<String, AttributeValue>) -> Result<String, String> {
    let mut obj = serde_json::Map::new();
    for (k, v) in map {
        obj.insert(k.clone(), attr_value_to_json(v)?);
    }
    serde_json::to_string(&serde_json::Value::Object(obj))
        .map_err(|e| format!("Serialization error: {}", e))
}

fn attr_value_to_json(av: &AttributeValue) -> Result<serde_json::Value, String> {
    if let Ok(s) = av.as_s() {
        return Ok(serde_json::Value::String(s.to_string()));
    }
    if let Ok(n) = av.as_n() {
        if let Ok(i) = n.parse::<i64>() {
            return Ok(serde_json::Value::Number(i.into()));
        }
        if let Ok(f) = n.parse::<f64>() {
            if let Some(num) = serde_json::Number::from_f64(f) {
                return Ok(serde_json::Value::Number(num));
            }
        }
        return Ok(serde_json::Value::String(n.to_string()));
    }
    if let Ok(b) = av.as_bool() {
        return Ok(serde_json::Value::Bool(*b));
    }
    if av.is_null() {
        return Ok(serde_json::Value::Null);
    }
    if let Ok(ss) = av.as_ss() {
        return Ok(serde_json::Value::Array(
            ss.iter()
                .map(|s| serde_json::Value::String(s.to_string()))
                .collect(),
        ));
    }
    if let Ok(ns) = av.as_ns() {
        return Ok(serde_json::Value::Array(
            ns.iter()
                .map(|s| serde_json::Value::String(s.to_string()))
                .collect(),
        ));
    }
    if let Ok(bs) = av.as_bs() {
        return Ok(serde_json::Value::Array(
            bs.iter()
                .map(|blob| {
                    serde_json::Value::String(format!("<blob:{} bytes>", blob.as_ref().len()))
                })
                .collect(),
        ));
    }
    if let Ok(l) = av.as_l() {
        let mut arr = Vec::new();
        for v in l {
            arr.push(attr_value_to_json(v)?);
        }
        return Ok(serde_json::Value::Array(arr));
    }
    if let Ok(m) = av.as_m() {
        let mut obj = serde_json::Map::new();
        for (k, v) in m {
            obj.insert(k.clone(), attr_value_to_json(v)?);
        }
        return Ok(serde_json::Value::Object(obj));
    }
    if let Ok(b) = av.as_b() {
        return Ok(serde_json::Value::String(format!(
            "<blob:{} bytes>",
            b.as_ref().len()
        )));
    }
    Err("Unsupported AttributeValue".to_string())
}

fn attr_value_kind(av: &AttributeValue) -> &'static str {
    if av.as_s().is_ok() {
        "string"
    } else if av.as_n().is_ok() {
        "number"
    } else if av.as_bool().is_ok() {
        "boolean"
    } else if av.is_null() {
        "null"
    } else if av.as_b().is_ok() {
        "binary"
    } else if av.as_ss().is_ok() {
        "string_set"
    } else if av.as_ns().is_ok() {
        "number_set"
    } else if av.as_bs().is_ok() {
        "binary_set"
    } else if av.as_l().is_ok() {
        "list"
    } else if av.as_m().is_ok() {
        "map"
    } else {
        "unknown"
    }
}

// ─── Filter expression builder ─────────────────────────────────────────────

fn filter_value_to_attr_value(value_type: &str, raw_value: &str) -> Result<AttributeValue, String> {
    match value_type {
        "string" => Ok(AttributeValue::S(raw_value.to_string())),
        "number" => {
            raw_value
                .parse::<f64>()
                .map_err(|_| format!("Invalid number '{}'", raw_value))?;
            Ok(AttributeValue::N(raw_value.to_string()))
        }
        "boolean" => {
            let v = raw_value
                .parse::<bool>()
                .map_err(|_| format!("Invalid boolean '{}'. Use true or false.", raw_value))?;
            Ok(AttributeValue::Bool(v))
        }
        "null" => Ok(AttributeValue::Null(true)),
        "binary" => Ok(AttributeValue::S(raw_value.to_string())),
        _ => Err(format!("Unsupported filter value_type '{}'", value_type)),
    }
}

fn build_filter_expression_parts(
    filters: &[FilterClause],
) -> Result<FilterExpressionParts, String> {
    let mut names: Vec<(String, String)> = Vec::new();
    let mut values: Vec<(String, AttributeValue)> = Vec::new();
    let mut terms: Vec<String> = Vec::new();

    for (i, f) in filters.iter().enumerate() {
        let attr = f.attribute.trim();
        if attr.is_empty() {
            continue;
        }

        let name_token = format!("#f{}", i);
        names.push((name_token.clone(), attr.to_string()));

        let term = match f.condition.as_str() {
            "eq" => {
                let vt = format!(":v{}", i);
                let raw = f
                    .value
                    .clone()
                    .ok_or("Missing filter value for 'Equal to'.")?;
                values.push((vt.clone(), filter_value_to_attr_value(&f.value_type, &raw)?));
                format!("{} = {}", name_token, vt)
            }
            "ne" => {
                let vt = format!(":v{}", i);
                let raw = f
                    .value
                    .clone()
                    .ok_or("Missing filter value for 'Not equal to'.")?;
                values.push((vt.clone(), filter_value_to_attr_value(&f.value_type, &raw)?));
                format!("{} <> {}", name_token, vt)
            }
            "lte" => {
                let vt = format!(":v{}", i);
                let raw = f.value.clone().ok_or("Missing filter value for '<='.")?;
                values.push((vt.clone(), filter_value_to_attr_value(&f.value_type, &raw)?));
                format!("{} <= {}", name_token, vt)
            }
            "lt" => {
                let vt = format!(":v{}", i);
                let raw = f.value.clone().ok_or("Missing filter value for '<'.")?;
                values.push((vt.clone(), filter_value_to_attr_value(&f.value_type, &raw)?));
                format!("{} < {}", name_token, vt)
            }
            "gte" => {
                let vt = format!(":v{}", i);
                let raw = f.value.clone().ok_or("Missing filter value for '>='.")?;
                values.push((vt.clone(), filter_value_to_attr_value(&f.value_type, &raw)?));
                format!("{} >= {}", name_token, vt)
            }
            "gt" => {
                let vt = format!(":v{}", i);
                let raw = f.value.clone().ok_or("Missing filter value for '>'.")?;
                values.push((vt.clone(), filter_value_to_attr_value(&f.value_type, &raw)?));
                format!("{} > {}", name_token, vt)
            }
            "between" => {
                let v1 = format!(":v{}", i);
                let v2 = format!(":w{}", i);
                let raw1 = f
                    .value
                    .clone()
                    .ok_or("Missing first value for 'Between'.")?;
                let raw2 = f
                    .value2
                    .clone()
                    .ok_or("Missing second value for 'Between'.")?;
                values.push((
                    v1.clone(),
                    filter_value_to_attr_value(&f.value_type, &raw1)?,
                ));
                values.push((
                    v2.clone(),
                    filter_value_to_attr_value(&f.value_type, &raw2)?,
                ));
                format!("{} BETWEEN {} AND {}", name_token, v1, v2)
            }
            "exists" => format!("attribute_exists({})", name_token),
            "not_exists" => format!("attribute_not_exists({})", name_token),
            "contains" => {
                let vt = format!(":v{}", i);
                let raw = f
                    .value
                    .clone()
                    .ok_or("Missing filter value for 'Contains'.")?;
                values.push((vt.clone(), filter_value_to_attr_value(&f.value_type, &raw)?));
                format!("contains({}, {})", name_token, vt)
            }
            "not_contains" => {
                let vt = format!(":v{}", i);
                let raw = f
                    .value
                    .clone()
                    .ok_or("Missing filter value for 'Not contains'.")?;
                values.push((vt.clone(), filter_value_to_attr_value(&f.value_type, &raw)?));
                format!("NOT contains({}, {})", name_token, vt)
            }
            "begins_with" => {
                let vt = format!(":v{}", i);
                let raw = f
                    .value
                    .clone()
                    .ok_or("Missing filter value for 'Begins with'.")?;
                values.push((vt.clone(), filter_value_to_attr_value(&f.value_type, &raw)?));
                format!("begins_with({}, {})", name_token, vt)
            }
            other => return Err(format!("Unsupported filter condition '{}'", other)),
        };

        terms.push(term);
    }

    Ok(FilterExpressionParts {
        expression: if terms.is_empty() {
            None
        } else {
            Some(terms.join(" AND "))
        },
        names,
        values,
    })
}
