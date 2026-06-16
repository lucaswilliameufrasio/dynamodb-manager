# Architecture

DynamoDB Manager follows a **two-layer architecture** with a clear FFI boundary:

```
┌──────────────┐     flutter_rust_bridge     ┌───────────────┐
│  Flutter UI  │ ◄──────────────────────────► │  Rust Backend │
│  (Dart)      │     (auto-generated FFI)     │  (cdylib)     │
└──────────────┘                              └──────┬────────┘
                                                     │
                                           ┌─────────▼────────┐
                                           │  AWS SDK for Rust │
                                           │  (aws-config +    │
                                           │   aws-sdk-dynamo) │
                                           └──────────────────┘
```

## UI Layer (Flutter/Dart)

The app has three main screens:

- **ProfileSelectionScreen** — landing screen that lists discovered AWS profiles, provides login/auth actions, diagnostics, and navigation to the workspace.
- **WorkspaceScreen** (in `workspace_controller.dart`) — the main table browser with table list, item scan/query, schema details, and filter controls.
- **DevLogsScreen** — a development-only panel that displays Rust-side log events in real time.

State is managed by `WorkspaceController`, a `ChangeNotifier` that holds the selected profile, table list, and current page of items.

## Rust Backend

The Rust side is organized into three API modules, each exposing functions via `flutter_rust_bridge`:

### `aws_profiles.rs`
- **Profile discovery** — reads `~/.aws/credentials` and `~/.aws/config`, classifies each profile by auth type
- **Auth** — runs `aws login`, `aws sso login`, or `aws configure sso` as subprocesses
- **Capability detection** — probes which subcommands the installed AWS CLI supports (3s timeouts)
- **Diagnostics** — returns file paths, existence flags, capability list, and error messages (no secrets)

### `dynamodb.rs`
- Builds a per-profile `aws-sdk-dynamodb` client with optional region/endpoint overrides
- All requests have timeouts: **8s for config resolution**, **15s for API operations**
- Exposes `list_tables`, `describe_table`, `scan_items`, `query_items`, `put_item`, `delete_item`, and `update_item`
- Filter clauses are parsed on the Rust side into `FilterExpression` + `ExpressionAttributeNames`/`Values`

### `dev_logs.rs`
- A static `Mutex<VecDeque<DevLogEntry>>` with capacity 300 entries
- Three log levels: `info`, `warn`, `error`
- Used internally by other Rust modules for instrumentation
- Exposed to Flutter as `get_recent_dev_logs()` and `clear_dev_logs()`

## macOS-Specific Notes

- The app sandbox entitlement has been removed (`com.apple.security.app-sandbox = NO`) to allow reading `~/.aws/*`
- Home directory is resolved via `getpwuid_r` instead of `$HOME` to avoid macOS container directory redirects
