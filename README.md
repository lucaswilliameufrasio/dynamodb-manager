# DynamoDB Manager

A **desktop DynamoDB browser and manager** built with Flutter (frontend) + Rust (backend) via `flutter_rust_bridge`. Intended for developers who want a GUI to inspect, scan, and manage DynamoDB tables without leaving their machine.

## Features

- **AWS profile discovery** — reads `~/.aws/credentials` and `~/.aws/config`; classifies profiles as `static`, `short_term`, `sso`, `role`, or `credential_source`
- **AWS CLI–based authentication** — supports `aws login`, `aws sso login`, and `aws configure sso`; detects available capabilities automatically
- **DynamoDB operations** — list/create/delete tables, create/edit/delete items, scan/query with filters and pagination; table creation supports sort keys, GSIs, and on-demand or provisioned billing
- **Diagnostics panel** — inspect profile paths, file existence, CLI capabilities, and error messages without exposing secrets
- **Dev Logs panel** — in-app view of Rust-side events (info, warning, error) for debugging

## Prerequisites

| Tool        | Recommended version | Notes                             |
|-------------|---------------------|-----------------------------------|
| Flutter     | `3.47.4`            | Pinned with FVM; macOS/Linux desktop |
| Rust        | `1.96.0`            | Stable toolchain                  |
| AWS CLI     | any recent          | Must be in `$PATH`                |
| Dart SDK    | `3.13.3`             | Bundled with the pinned Flutter SDK |

## Quick Start

```bash
# 1. Install all dependencies
mise run setup

# 2. Run the app
mise run run
```

After login, the app reads `~/.aws/credentials` and `~/.aws/config` to list your profiles.
Click a profile to open the workspace, where you can list tables, scan items, and inspect table metadata.

Or, without `mise`:

```bash
flutter pub get && cd rust_builder && cargo fetch && cd ..
flutter run -d macos
```

## Development Commands

Available via `mise` (or `make`):

| Command             | Description                            |
|---------------------|----------------------------------------|
| `mise run setup`    | Install Flutter + Rust dependencies    |
| `mise run doctor`   | Run Flutter doctor + Rust version check|
| `mise run info`     | Show project info and tool versions    |
| `mise run run`      | Run the app on macOS                   |
| `mise run sync`     | Regenerate FRB bindings (after Rust changes) |
| `mise run test`     | Run Flutter tests + Rust tests         |
| `mise run lint`     | Run Flutter analyze + Rust clippy      |
| `mise run fmt`      | Format Dart + Rust code                |
| `mise run upgrade`  | Upgrade all dependencies               |
| `mise run clean`    | Clean all build artifacts              |
| `mise run check`    | Format → lint → test pipeline          |
| `mise run bootstrap`| Full environment setup for new devs    |
| `mise run aws-check`| Validate AWS CLI and profiles          |

All commands also work with `make <command>` (e.g. `make setup`, `make sync`).

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
- [Performance](docs/performance.md) — Reproducible local baselines for Flutter and Rust
- [Distribution](docs/distribution.md) — Desktop release workflow and packages

## Desktop distribution

Tagged releases are packaged for macOS (Apple Silicon and Intel) and Linux
(x86_64). The release workflow publishes a macOS DMG and `.app.zip` for each
architecture, plus a Linux AppImage.

To package locally, use `make package-dist` on macOS or Linux. For a release,
run **Actions → Prepare Release** on `main`, merge the generated version PR, and
push its matching `vX.Y.Z` tag. Downloads and macOS signing details are in
[Distribution](docs/distribution.md) and [macOS signing](docs/macos-signing.md).
