# Development

## Prerequisites

| Tool | Recommended version | Install |
|------|-------------------|---------|
| Flutter | `3.44.2` | `brew install flutter` or [flutter.dev](https://flutter.dev) |
| Rust | `1.96.0` | `rustup install 1.96.0 && rustup default 1.96.0` |
| AWS CLI | latest | `brew install awscli` |
| mise | latest | `brew install mise` (optional — for dev commands) |

Ensure the `macos` desktop target is available:

```bash
flutter config --enable-macos-desktop
```

## Development Commands

All commands are defined in `mise.toml` and mirrored as `make` targets.

### `mise run setup`
Install Flutter and Rust dependencies:
- `flutter pub get`
- `cargo fetch` in `rust_builder/`

### `mise run doctor`
Run `flutter doctor -v` and print `rustc`/`cargo` versions.

### `mise run info`
Show project metadata and all tool versions (Flutter, Dart, Rust, Cargo, AWS CLI).

### `mise run run`
Run the app on macOS:
```bash
flutter run -d macos
```

### `mise run sync`
Regenerate FRB bindings after Rust API changes:
```bash
flutter_rust_bridge_codegen generate
```

### `mise run test`
Run Flutter tests and (if any) Rust tests.

### `mise run lint`
Run `flutter analyze` and `cargo clippy`.

### `mise run fmt`
Format all Dart (via `dart format .`) and Rust (via `cargo fmt`) code.

### `mise run upgrade`
Upgrade all dependencies:
- `flutter pub upgrade --major-versions`
- `cargo update`

### `mise run clean`
Remove all build artifacts (`flutter clean` + `cargo clean`).

### `mise run check`
Full pipeline: format → lint → test.

### `mise run bootstrap`
Full environment setup for new contributors:
- Set Rust toolchain and macOS target
- Enable macOS Flutter desktop
- Install dependencies
- Print next steps

### `mise run aws-check`
Validate the AWS CLI installation and list configured profiles.

### Without `mise`

All commands also work as `make <target>` (e.g. `make sync`). If neither `mise` nor `make` is available, run the underlying commands directly:

```bash
flutter pub get && cd rust_builder && cargo fetch && cd ..
flutter run -d macos
```

## macOS Sandbox

The app sandbox has been **disabled** in both `DebugProfile.entitlements` and `Release.entitlements` because reading `~/.aws/*` requires file-system access outside the sandbox.

If you re-enable sandboxing, home directory resolution will use `getpwuid_r` but the app may not be able to read AWS config files.

## Key Source Locations

| What | Path |
|------|------|
| Flutter entry point | `lib/main.dart` |
| App widget | `lib/src/app.dart` |
| Profile screen | `lib/src/screens/profile_selection_screen.dart` |
| Dev logs screen | `lib/src/screens/dev_logs_screen.dart` |
| Metrics modal | `lib/src/screens/profile_selection_screen.dart` (in `_TableDashboard`) |
| Workspace controller | `lib/src/controllers/workspace_controller.dart` |
| FRB-generated Dart bindings | `lib/src/rust/api/*.dart` |
| Rust API: profiles/auth | `rust/src/api/aws_profiles.rs` |
| Rust API: DynamoDB ops | `rust/src/api/dynamodb.rs` |
| Rust API: dev logs | `rust/src/api/dev_logs.rs` |

## Dev Logs Panel

During development, tap the **Dev Logs** button (in the profile screen app bar) to open a scrollable view of Rust-side events. This is useful for debugging profile discovery, auth, and DynamoDB operations. The buffer holds the last 300 entries.
