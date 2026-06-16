# DynamoDB Manager

A **desktop DynamoDB browser and manager** built with Flutter (frontend) + Rust (backend) via `flutter_rust_bridge`. Intended for developers who want a GUI to inspect, scan, and manage DynamoDB tables without leaving their machine.

## Features

- **AWS profile discovery** — reads `~/.aws/credentials` and `~/.aws/config`; classifies profiles as `static`, `short_term`, `sso`, `role`, or `credential_source`
- **AWS CLI–based authentication** — supports `aws login`, `aws sso login`, and `aws configure sso`; detects available capabilities automatically
- **DynamoDB operations** — list tables, describe schema, scan/query items with filters, pagination
- **Diagnostics panel** — inspect profile paths, file existence, CLI capabilities, and error messages without exposing secrets
- **Dev Logs panel** — in-app view of Rust-side events (info, warning, error) for debugging

## Prerequisites

| Tool        | Recommended version | Notes                             |
|-------------|---------------------|-----------------------------------|
| Flutter     | `3.44.2`            | macOS desktop target required     |
| Rust        | `1.96.0`            | Stable toolchain                  |
| AWS CLI     | any recent          | Must be in `$PATH`                |
| Dart SDK    | `^3.12.1`           | Managed by Flutter SDK            |

## Quick Start

```bash
# 1. Install dependencies
flutter pub get
cd rust_builder && cargo fetch && cd ..

# 2. Run the app
flutter run -d macos
```

After login, the app reads `~/.aws/credentials` and `~/.aws/config` to list your profiles.
Click a profile to open the workspace, where you can list tables, scan items, and inspect table metadata.

## Regenerating FFI bindings

After changing Rust API code, regenerate the Dart bindings:

```bash
flutter_rust_bridge_codegen generate && flutter run
```

## Project Structure

```
lib/
├── main.dart                          # Entry point
├── src/
│   ├── app.dart                       # MaterialApp, theme, routing
│   ├── screens/
│   │   ├── profile_selection_screen.dart  # Profile list, auth, diagnostics
│   │   └── dev_logs_screen.dart           # Rust log viewer
│   ├── controllers/
│   │   └── workspace_controller.dart      # Profile/table/item state
│   ├── models/
│   │   ├── dynamo_table.dart              # Table summary model
│   │   └── dynamo_item.dart               # Item model
│   └── rust/api/
│       ├── aws_profiles.dart         # FRB-generated: profile/auth bindings
│       ├── dynamodb.dart              # FRB-generated: DynamoDB bindings
│       └── dev_logs.dart              # FRB-generated: dev logs bindings
rust/src/
├── lib.rs
└── api/
    ├── mod.rs
    ├── aws_profiles.rs                # Profile discovery, auth, capabilities
    ├── dynamodb.rs                    # DynamoDB client, scan, describe
    └── dev_logs.rs                    # In-memory bounded log buffer
```

## Documentation

- [Architecture](docs/architecture.md) — Flutter ↔ Rust ↔ AWS SDK flow
- [AWS Auth](docs/aws-auth.md) — Profile discovery, login, diagnostics
- [Development](docs/development.md) — Workflow, codegen, key modules
- [Troubleshooting](docs/troubleshooting.md) — Common issues and fixes
- [Security](docs/security.md) — What the app logs and never exposes
