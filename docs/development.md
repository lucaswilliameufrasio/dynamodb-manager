# Development

## Prerequisites

| Tool | Recommended version | Install |
|------|-------------------|---------|
| Flutter | `3.44.2` | `brew install flutter` or [flutter.dev](https://flutter.dev) |
| Rust | `1.96.0` | `rustup install 1.96.0 && rustup default 1.96.0` |
| AWS CLI | latest | `brew install awscli` |

Ensure the `macos` desktop target is available:

```bash
flutter config --enable-macos-desktop
```

## Running the App

```bash
# First time or after dep changes
flutter pub get
cd rust_builder && cargo fetch && cd ..

# Run on macOS
flutter run -d macos

# Run with release build (no debug overlay)
flutter run -d macos --release
```

## Regenerating FFI Bindings

After changing any Rust API function signatures or adding new public functions:

```bash
flutter_rust_bridge_codegen generate
```

This regenerates `lib/src/rust/api/*.dart` and `rust/src/frb_generated.rs`.
If codegen succeeds, run the app with `flutter run`.

## Key Source Locations

| What | Path |
|------|------|
| Flutter entry point | `lib/main.dart` |
| App widget | `lib/src/app.dart` |
| Profile screen | `lib/src/screens/profile_selection_screen.dart` |
| Dev logs screen | `lib/src/screens/dev_logs_screen.dart` |
| Workspace controller | `lib/src/controllers/workspace_controller.dart` |
| FRB-generated Dart bindings | `lib/src/rust/api/*.dart` |
| Rust API: profiles/auth | `rust/src/api/aws_profiles.rs` |
| Rust API: DynamoDB ops | `rust/src/api/dynamodb.rs` |
| Rust API: dev logs | `rust/src/api/dev_logs.rs` |

## Dev Logs Panel

During development, tap the **Dev Logs** button (in the profile screen app bar) to open a scrollable view of Rust-side events. This is useful for debugging profile discovery, auth, and DynamoDB operations. The buffer holds the last 300 entries.

## macOS Sandbox

The app sandbox has been **disabled** in both `DebugProfile.entitlements` and `Release.entitlements` because reading `~/.aws/*` requires file-system access outside the sandbox.

If you re-enable sandboxing, home directory resolution will use `getpwuid_r` but the app may not be able to read AWS config files.
