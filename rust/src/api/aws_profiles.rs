use ini::Ini;
use std::collections::HashMap;
use std::path::PathBuf;

/// Represents a discovered AWS profile.
#[derive(Debug, Clone)]
pub struct AwsProfile {
    pub name: String,
    /// One of: "sso", "role", "credential_source", "static", "short_term"
    pub kind: String,
}

/// Diagnostics to help debug profile discovery and CLI setup.
#[derive(Debug, Clone)]
pub struct AwsDiagnostics {
    pub config_path: String,
    pub credentials_path: String,
    pub config_exists: bool,
    pub credentials_exists: bool,
    pub capabilities: Vec<String>,
    pub profile_count: i32,
    pub errors: Vec<String>,
}

fn home_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        return Err("Windows is not supported for profile discovery".to_string());
    }

    // Use getpwuid_r to resolve the real user home directory,
    // bypassing macOS sandbox redirects to Library/Containers/...
    let uid = unsafe { libc::getuid() };
    let bufsize = match unsafe { libc::sysconf(libc::_SC_GETPW_R_SIZE_MAX) } {
        n if n > 0 => n as usize,
        _ => 4096,
    };
    let mut buf = vec![0u8; bufsize];
    let mut pwd: libc::passwd = unsafe { std::mem::zeroed() };
    let mut result: *mut libc::passwd = std::ptr::null_mut();

    let ret = unsafe {
        libc::getpwuid_r(uid, &mut pwd, buf.as_mut_ptr() as *mut i8, bufsize, &mut result)
    };

    if ret == 0 && !result.is_null() {
        let home = unsafe { std::ffi::CStr::from_ptr(pwd.pw_dir) };
        let home_str = home.to_str().unwrap_or("");
        if !home_str.is_empty() && PathBuf::from(home_str).is_dir() {
            return Ok(PathBuf::from(home_str));
        }
    }

    // Fallback to HOME env var
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "HOME environment variable is not set and getpwuid_r failed".to_string())
}

fn aws_credentials_path() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("AWS_SHARED_CREDENTIALS_FILE") {
        return Ok(PathBuf::from(path));
    }
    Ok(home_dir()?.join(".aws").join("credentials"))
}

fn aws_config_path() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("AWS_CONFIG_FILE") {
        return Ok(PathBuf::from(path));
    }
    Ok(home_dir()?.join(".aws").join("config"))
}

fn normalize_config_profile_name(section: &str) -> String {
    section.strip_prefix("profile ").unwrap_or(section).trim().to_string()
}

// ─── Capability detection ─────────────────────────────────────────────────

async fn probe_subcommand(args: &[&str], timeout_secs: u64) -> bool {
    let Ok(mut child) = tokio::process::Command::new("aws")
        .args(args)
        .env("AWS_PAGER", "")
        .env("PAGER", "cat")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    else {
        return false;
    };
    match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), child.wait()).await {
        Ok(Ok(status)) => status.success(),
        _ => false,
    }
}

/// Detect which AWS CLI auth subcommands the installed CLI supports.
/// Each probe has a 3-second timeout to prevent hanging on CLI help pages.
pub async fn get_aws_cli_capabilities() -> Result<Vec<String>, String> {
    let has_cli = tokio::process::Command::new("which")
        .arg("aws")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);

    if !has_cli {
        return Ok(Vec::new());
    }

    let mut caps = Vec::new();
    if probe_subcommand(&["login", "help"], 3).await {
        caps.push("aws_login".to_string());
    }
    if probe_subcommand(&["sso", "login", "help"], 3).await {
        caps.push("sso_login".to_string());
    }
    if probe_subcommand(&["configure", "sso", "help"], 3).await {
        caps.push("configure_sso".to_string());
    }
    Ok(caps)
}

// ─── Diagnostics ──────────────────────────────────────────────────────────

/// Returns diagnostics about the AWS CLI environment and profile files.
/// Never returns secret values, only paths, existence, and metadata.
/// Subprocess probes have a 3-second timeout.
pub async fn get_aws_diagnostics() -> AwsDiagnostics {
    let mut errors: Vec<String> = Vec::new();
    let mut capabilities: Vec<String> = Vec::new();

    let has_cli = tokio::process::Command::new("which")
        .arg("aws")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);

    if has_cli {
        if probe_subcommand(&["login", "help"], 3).await {
            capabilities.push("aws_login".to_string());
        }
        if probe_subcommand(&["sso", "login", "help"], 3).await {
            capabilities.push("sso_login".to_string());
        }
        if probe_subcommand(&["configure", "sso", "help"], 3).await {
            capabilities.push("configure_sso".to_string());
        }
    } else {
        errors.push("AWS CLI not found in PATH. Install it first.".to_string());
    }

    let creds_path_str = aws_credentials_path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|e| format!("(error: {})", e));
    let config_path_str = aws_config_path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|e| format!("(error: {})", e));

    let creds_exists = PathBuf::from(&creds_path_str).exists();
    let config_exists = PathBuf::from(&config_path_str).exists();

    let profile_count = list_local_aws_profiles()
        .map(|p| p.len() as i32)
        .unwrap_or(0);

    AwsDiagnostics {
        config_path: config_path_str,
        credentials_path: creds_path_str,
        config_exists,
        credentials_exists: creds_exists,
        capabilities,
        profile_count,
        errors,
    }
}

// ─── Profile discovery ────────────────────────────────────────────────────

/// List all local AWS profiles, respecting AWS_CONFIG_FILE and
/// AWS_SHARED_CREDENTIALS_FILE environment variables.
pub fn list_local_aws_profiles() -> Result<Vec<AwsProfile>, String> {
    let mut profiles: HashMap<String, String> = HashMap::new();

    // 1) Parse credentials file
    let creds_path = aws_credentials_path()?;
    if creds_path.exists() {
        if let Ok(ini) = Ini::load_from_file(&creds_path) {
            for (section, props) in ini.iter() {
                let name = section.unwrap_or("default").to_string();
                let has_key = props.contains_key("aws_access_key_id");
                let has_secret = props.contains_key("aws_secret_access_key");
                let has_token = props.contains_key("aws_session_token");
                if has_key && has_secret {
                    let kind = if has_token { "short_term" } else { "static" };
                    profiles.entry(name).or_insert_with(|| kind.to_string());
                }
            }
        }
    }

    // 2) Parse config file for richer classification
    let config_path = aws_config_path()?;
    if config_path.exists() {
        if let Ok(ini) = Ini::load_from_file(&config_path) {
            for (section, props) in ini.iter() {
                let section_name = section.unwrap_or("default").to_string();
                let profile_name = normalize_config_profile_name(&section_name);

                if section_name.starts_with("sso-session ") || section_name.starts_with("services ") {
                    continue;
                }

                let kind = if props.contains_key("sso_session") || props.contains_key("sso_start_url") {
                    "sso".to_string()
                } else if props.contains_key("role_arn") && props.contains_key("credential_source") {
                    "credential_source".to_string()
                } else if props.contains_key("role_arn") {
                    "role".to_string()
                } else {
                    profiles.get(&profile_name).cloned().unwrap_or_else(|| "static".to_string())
                };

                profiles.entry(profile_name).or_insert(kind);
            }
        }
    }

    let mut out: Vec<AwsProfile> = profiles
        .into_iter()
        .map(|(name, kind)| AwsProfile { name, kind })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

// ─── Auth functions ───────────────────────────────────────────────────────

async fn check_aws_cli() -> Result<(), String> {
    let which = tokio::process::Command::new("which")
        .arg("aws")
        .output()
        .await
        .map_err(|_| "Failed to check for AWS CLI".to_string())?;
    if !which.status.success() {
        return Err("AWS CLI not found. Please install it first.".to_string());
    }
    Ok(())
}

/// Authenticate a non-SSO profile via `aws login --profile <name>`.
pub async fn aws_login(profile_name: String) -> Result<String, String> {
    check_aws_cli().await?;
    let status = tokio::process::Command::new("aws")
        .arg("login")
        .arg("--profile")
        .arg(&profile_name)
        .status()
        .await
        .map_err(|e| format!("Failed to run `aws login`: {}", e))?;

    if status.success() {
        Ok(format!("AWS login successful for profile '{}'", profile_name))
    } else {
        Err(format!(
            "AWS login failed for profile '{}' (exit code: {:?})",
            profile_name, status.code()
        ))
    }
}

/// Authenticate an SSO profile via `aws sso login --profile <name>`.
pub async fn sso_login(profile_name: String) -> Result<String, String> {
    check_aws_cli().await?;
    let status = tokio::process::Command::new("aws")
        .arg("sso")
        .arg("login")
        .arg("--profile")
        .arg(&profile_name)
        .status()
        .await
        .map_err(|e| format!("Failed to run `aws sso login`: {}", e))?;

    if status.success() {
        Ok(format!("SSO login successful for profile '{}'", profile_name))
    } else {
        Err(format!(
            "SSO login failed for profile '{}' (exit code: {:?})",
            profile_name, status.code()
        ))
    }
}

/// Check whether the given profile has valid (non-expired) credentials.
pub async fn check_profile_credentials(profile_name: String) -> Result<bool, String> {
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .profile_name(&profile_name)
        .load()
        .await;
    let client = aws_sdk_dynamodb::Client::new(&config);

    match client.list_tables().limit(1).send().await {
        Ok(_) => Ok(true),
        Err(e) => {
            let err = e.to_string().to_lowercase();
            if err.contains("expiredtoken")
                || err.contains("credentials expired")
                || err.contains("expired")
                || (err.contains("sso") && err.contains("token"))
                || err.contains("credentials")
            {
                Ok(false)
            } else {
                Ok(true)
            }
        }
    }
}

// ─── Profile CRUD ─────────────────────────────────────────────────────────

/// Add an SSO profile by writing to ~/.aws/config.
pub fn add_sso_profile(
    name: String,
    sso_session: String,
    sso_start_url: String,
    sso_region: String,
    sso_account_id: String,
    sso_role_name: String,
) -> Result<(), String> {
    let config_path = aws_config_path()?;
    ensure_aws_dir(&config_path)?;

    let mut ini = if config_path.exists() {
        Ini::load_from_file(&config_path).map_err(|e| format!("Failed to load config: {}", e))?
    } else {
        Ini::new()
    };

    let profile_section = if name == "default" {
        name.clone()
    } else {
        format!("profile {}", name)
    };

    ini.with_section(Some(profile_section))
        .set("sso_session", &sso_session)
        .set("sso_start_url", &sso_start_url)
        .set("sso_region", &sso_region)
        .set("sso_account_id", &sso_account_id)
        .set("sso_role_name", &sso_role_name);

    let session_section = format!("sso-session {}", sso_session);
    ini.with_section(Some(session_section))
        .set("sso_region", &sso_region)
        .set("sso_start_url", &sso_start_url)
        .set("sso_registration_scopes", "sso:account:access");

    ini.write_to_file(&config_path)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

/// Delete a profile from both credential files.
pub fn delete_profile(name: String) -> Result<(), String> {
    let creds_path = aws_credentials_path()?;
    let config_path = aws_config_path()?;

    fn remove_section_from_file(path: &PathBuf, target_section: &str) -> Result<(), String> {
        if !path.exists() {
            return Ok(());
        }
        let ini = Ini::load_from_file(path)
            .map_err(|e| format!("Failed to load {}: {}", path.display(), e))?;
        let mut found = false;
        let mut out = Ini::new();
        for (section, props) in ini.iter() {
            let section_name = section.unwrap_or("default");
            if section_name == target_section {
                found = true;
                continue;
            }
            for (k, v) in props.iter() {
                out.with_section(Some(section_name.to_string())).set(k, v);
            }
        }
        if !found {
            return Ok(());
        }
        out.write_to_file(path)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
        Ok(())
    }

    remove_section_from_file(&creds_path, &name)?;
    let config_section = if name == "default" {
        name.clone()
    } else {
        format!("profile {}", name)
    };
    remove_section_from_file(&config_path, &config_section)?;
    Ok(())
}

fn ensure_aws_dir(path: &PathBuf) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create ~/.aws directory: {}", e))?;
    }
    Ok(())
}
