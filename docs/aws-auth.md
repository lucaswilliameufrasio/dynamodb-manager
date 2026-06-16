# AWS Authentication

The app uses the **AWS CLI** for authentication and reads local credential/config files for profile discovery. It never stores, logs, or transmits credentials itself.

## Profile Discovery

On startup, `list_local_aws_profiles()` parses two files:

| File | Default location | Env override |
|------|------------------|--------------|
| Credentials | `~/.aws/credentials` | `AWS_SHARED_CREDENTIALS_FILE` |
| Config | `~/.aws/config` | `AWS_CONFIG_FILE` |

Home directory is resolved via `getpwuid_r` (macOS) to bypass container/sandbox path redirects.

### Profile Classification

Each discovered profile gets a `kind` label:

| Kind | How detected |
|------|-------------|
| `static` | Has `aws_access_key_id` + `aws_secret_access_key` in credentials file |
| `short_term` | Same as static but also has `aws_session_token` |
| `sso` | Config section has `sso_session` or `sso_start_url` |
| `role` | Config section has `role_arn` without `credential_source` |
| `credential_source` | Config section has both `role_arn` and `credential_source` |

## Capability Detection

Before showing login options, the app probes the installed AWS CLI:

```bash
aws login help        # → capability "aws_login"
aws sso login help    # → capability "sso_login"
aws configure sso help # → capability "configure_sso"
```

Each probe has a **3-second timeout** and runs with `AWS_PAGER=""` to prevent hanging.

## Auth Methods

### aws login (non-SSO)
Opens the AWS IAM sign-in page in the browser for the selected profile.

### sso login
Runs `aws sso login --profile <name>` for SSO-enabled profiles.

### configure sso
Opens `aws configure sso` in the terminal for profiles that need SSO setup.

## Diagnostics

The diagnostics panel (`get_aws_diagnostics()`) exposes:

- Paths to `config` and `credentials` files
- Whether each file exists
- Detected AWS CLI capabilities
- Count of discovered profiles
- Error messages (e.g., "AWS CLI not found")

**It never includes**: access keys, secret keys, session tokens, profile contents, or any other sensitive data.
