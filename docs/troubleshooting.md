# Troubleshooting

## "AWS CLI not found" or empty profiles

1. Verify the AWS CLI is installed and in `$PATH`:
   ```bash
   which aws
   ```
2. Confirm you have profiles configured:
   ```bash
   aws configure list-profiles
   ```
3. If profiles exist but the app shows none, open the **Diagnostics** dialog to check file paths and existence.

## No profiles discovered

- Ensure `~/.aws/credentials` and/or `~/.aws/config` exist with valid content
- Check for environment variables `AWS_CONFIG_FILE` and `AWS_SHARED_CREDENTIALS_FILE` that may point elsewhere
- The **Diagnostics** button shows exact paths the app is trying to read

## macOS "cannot open" sandbox errors

This app has sandbox **disabled**. If you see sandbox-related errors:
1. Check `macos/Runner/DebugProfile.entitlements` and `macos/Runner/Release.entitlements` — `com.apple.security.app-sandbox` should be `false`
2. Rebuild with `flutter clean && flutter run -d macos`

## DynamoDB operation times out

- Config resolution times out after **8 seconds**
- Request operations time out after **15 seconds**
- Common causes:
  - AWS config is slow to resolve (complex SSO or role chains)
  - Network issues
  - The profile has expired credentials
- Check the **Dev Logs** panel for timeout messages with the profile name

## `flutter_rust_bridge_codegen generate` fails

- Ensure `flutter_rust_bridge_codegen` is installed:
  ```bash
  dart pub global activate flutter_rust_bridge_codegen
  ```
- Verify the Rust toolchain matches `rust-toolchain.toml` or use `rustup default 1.96.0`
- Check for syntax errors in `rust/src/api/*.rs`
- Re-run with verbose output:
  ```bash
  flutter_rust_bridge_codegen generate -v
  ```

## App crashes on launch

- Run from terminal to see stack traces: `flutter run -d macos`
- Check the **Dev Logs** panel for early errors (it's populated during `initState`)
- If the issue is profile-related, delete the problem profile and restart
