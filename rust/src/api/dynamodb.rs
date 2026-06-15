use crate::api::dev_logs::{log_info, log_error};
use aws_config::{BehaviorVersion, Region};
use aws_sdk_dynamodb::types::AttributeValue;
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
    log_info("dynamodb", format!("build_ddb_client start profile='{}'", profile));
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
    .await.map_err(|e| {
        log_error("dynamodb", format!("config timeout profile='{}': {}", profile, e));
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
    log_info("dynamodb", format!("list_tables start profile='{}'", profile));
    let client =
        build_ddb_client(&profile, region_override, endpoint_override).await?;
    let mut tables = Vec::new();
    let mut start = None;

    loop {
        let mut req = client.list_tables().limit(100);
        if let Some(s) = start {
            req = req.exclusive_start_table_name(s);
        }

        let result = with_timeout(
            async {
                req.send()
                    .await
                    .map_err(|e| e.to_string())
            },
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
    log_info("dynamodb", format!("list_tables done count={} profile='{}'", tables.len(), profile));
    Ok(tables)
}

pub async fn describe_table(
    profile: String,
    region_override: Option<String>,
    endpoint_override: Option<String>,
    table_name: String,
) -> Result<TableSummary, String> {
    log_info("dynamodb", format!("describe_table start table='{}' profile='{}'", table_name, profile));
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
    log_info("dynamodb", format!("scan_items start table='{}' profile='{}'", table_name, profile));
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
        async {
            request
                .send()
                .await
                .map_err(|e| e.to_string())
        },
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

    log_info("dynamodb", format!("scan_items done count={} table='{}'", items_json.len(), table_name));

    Ok(ItemsPageResult {
        items_json,
        last_evaluated_key_json,
    })
}

/// Query items with partition-key filter, pagination and optional filter expressions.
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
    log_info("dynamodb", format!("query_items start table='{}' profile='{}'", table_name, profile));
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
        async {
            request
                .send()
                .await
                .map_err(|e| e.to_string())
        },
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

    log_info("dynamodb", format!("query_items done count={} table='{}'", items_json.len(), table_name));

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
    log_info("dynamodb", format!("put_item start table='{}' profile='{}'", table_name, profile));
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
    log_info("dynamodb", format!("put_item_create_only start table='{}'", table_name));
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
    log_info("dynamodb", format!("put_item_update_only start table='{}'", table_name));
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
    log_info("dynamodb", format!("delete_item start table='{}' profile='{}'", table_name, profile));
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
    log_info("dynamodb", format!("list_table_attributes start table='{}'", table_name));
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
                let raw1 = f.value.clone().ok_or("Missing first value for 'Between'.")?;
                let raw2 = f.value2.clone().ok_or("Missing second value for 'Between'.")?;
                values.push((v1.clone(), filter_value_to_attr_value(&f.value_type, &raw1)?));
                values.push((v2.clone(), filter_value_to_attr_value(&f.value_type, &raw2)?));
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
