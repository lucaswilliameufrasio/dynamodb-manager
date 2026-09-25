# Security

## Credential Handling

The app code **never**:

- Persists, manually copies/caches, or logs AWS access keys, secret keys, or session tokens
- Transmits credentials over the network (all AWS SDK calls go directly to the AWS endpoint)
- Reads or exposes raw credential file contents in the UI

All authentication is delegated to the **AWS CLI** or the **AWS SDK for Rust**, which manage credentials according to the [standard AWS credential chain](https://docs.aws.amazon.com/sdkref/latest/guide/standardized-credentials.html).

The DynamoDB manager reuses AWS SDK client instances in memory for up to five
minutes per profile/region/endpoint. Resolved credentials remain under the
AWS SDK credential provider's in-memory refresh/cache behavior; credential
values are not copied into the app's own cache or written to disk. Successful
in-app `aws login` and `aws sso login` invalidate that profile's client cache.

## Diagnostics

The diagnostics panel (`get_aws_diagnostics()`) exposes only:

- File paths (e.g., `/Users/you/.aws/config`)
- File existence (yes/no)
- AWS CLI capabilities detected (`aws_login`, `sso_login`, `configure_sso`)
- Count of discovered profiles
- Error messages such as "AWS CLI not found"

It does **not** expose:
- Profile names of credential-bearing profiles (only the count)
- Access keys, secret keys, or tokens
- Contents of `~/.aws/credentials` or `~/.aws/config`

## Dev Logs

The Rust-side dev log buffer (`dev_logs.rs`) stores up to 300 entries with fields: timestamp, level, scope, and message. The messages are intentionally high-level (e.g., "profile discovery start", "found 3 profiles", "config loaded profile='default'"). They never include raw credentials, SDK error details that may contain secrets, or file contents.

The buffer is in-memory only — it is not persisted to disk and is lost when the app exits.

## Best Practices

- Use `aws configure` or `aws sso login` to manage credentials rather than editing files manually
- If you share screenshots or recordings of the app, avoid including the Diagnostics panel if it shows file paths you consider sensitive
- The app does not implement its own network communication; all AWS traffic goes through the official `aws-sdk-dynamodb` crate
