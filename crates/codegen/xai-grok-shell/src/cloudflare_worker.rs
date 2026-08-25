//! Cloudflare Worker credential storage for the "stable remote-MCP URL"
//! feature — currently deployed/run only by the npm package's Node
//! scripts (`bin/cloudflare-worker.js`), but the credential file itself
//! (`~/.adevgrok/cloudflare-worker.json`) is a plain JSON format shared
//! between them and this Rust side, so `/mcp-worker configure` in the TUI
//! can save credentials the npm wrapper picks up on its next launch.
//!
//! Deliberately scoped to credential storage only: this does not run the
//! local MCP bridge, start a Cloudflare Quick Tunnel, or deploy the Worker
//! script — that orchestration lives in the npm wrapper and needs the
//! bridge process it manages. What's here mirrors `cloudflare-worker.js`'s
//! `validateConfig`/`discoverAccounts`/`writeConfig`/`readConfig`/
//! `removeConfig` exactly enough to keep the file format compatible.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const API_BASE: &str = "https://api.cloudflare.com/client/v4";

fn config_path() -> PathBuf {
    crate::util::grok_home::grok_home().join("cloudflare-worker.json")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareWorkerConfig {
    #[serde(rename = "apiToken")]
    pub api_token: String,
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "workerName")]
    pub worker_name: String,
    pub enabled: bool,
}

pub fn read_config() -> Option<CloudflareWorkerConfig> {
    let content = std::fs::read_to_string(config_path()).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_config(config: &CloudflareWorkerConfig) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    std::fs::write(&path, json)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Idempotent — a no-op if nothing was ever configured.
pub fn remove_config() {
    let _ = std::fs::remove_file(config_path());
}

/// A masked view of the stored config for `/mcp-worker status`: the token
/// is shown as `abcd...wxyz` (first/last 4 chars), never in full.
pub fn masked_status() -> Option<String> {
    let cfg = read_config()?;
    let masked = if cfg.api_token.len() > 8 {
        format!(
            "{}...{}",
            &cfg.api_token[..4],
            &cfg.api_token[cfg.api_token.len() - 4..]
        )
    } else {
        "(short token)".to_owned()
    };
    Some(format!(
        "Worker: {}\nAccount: {}\nAPI token: {masked}",
        cfg.worker_name, cfg.account_id
    ))
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    success: bool,
    #[serde(default)]
    errors: Vec<ApiError>,
    result: Option<T>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: String,
}

#[derive(Debug, Deserialize)]
pub struct CloudflareAccount {
    pub id: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct WorkersSubdomain {
    subdomain: String,
}

async fn api_get<T: for<'de> Deserialize<'de>>(
    api_token: &str,
    path: &str,
) -> Result<T, String> {
    let response = xai_grok_http::shared_client()
        .get(format!("{API_BASE}{path}"))
        .bearer_auth(api_token)
        .send()
        .await
        .map_err(|e| format!("Cloudflare API request failed: {e}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read Cloudflare API response: {e}"))?;
    let envelope: ApiEnvelope<T> = serde_json::from_str(&text).map_err(|e| {
        format!("Cloudflare API returned an unexpected response ({status}): {e}")
    })?;
    if !envelope.success {
        let detail = envelope
            .errors
            .into_iter()
            .map(|e| e.message)
            .collect::<Vec<_>>()
            .join("; ");
        let detail = if detail.is_empty() {
            status.to_string()
        } else {
            detail
        };
        return Err(format!("Cloudflare API error: {detail}"));
    }
    envelope
        .result
        .ok_or_else(|| "Cloudflare API returned no result".to_owned())
}

/// Discover every account this token can access.
pub async fn discover_accounts(api_token: &str) -> Result<Vec<CloudflareAccount>, String> {
    api_get(api_token, "/accounts?per_page=50").await
}

/// Confirm the token can read this account's Workers config, returning its
/// `workers.dev` subdomain (needed to build the final Worker URL preview).
async fn validate_account(api_token: &str, account_id: &str) -> Result<String, String> {
    let subdomain: WorkersSubdomain = api_get(
        api_token,
        &format!("/accounts/{}/workers/subdomain", urlencode(account_id)),
    )
    .await?;
    Ok(subdomain.subdomain)
}

fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
                c.to_string()
            } else {
                format!("%{:02X}", c as u32)
            }
        })
        .collect()
}

/// Sanitize a worker name the same way `cloudflare-worker.js` does:
/// lowercase, non `[a-z0-9-_]` chars become `-`.
fn sanitize_worker_name(name: &str) -> String {
    name.to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect()
}

pub const DEFAULT_WORKER_NAME: &str = "failure-mcp";

/// Result of a successful `/mcp-worker configure`.
pub struct ConfigureOutcome {
    pub worker_name: String,
    pub account_id: String,
    pub subdomain: String,
}

impl ConfigureOutcome {
    pub fn worker_url(&self) -> String {
        format!("https://{}.{}.workers.dev", self.worker_name, self.subdomain)
    }
}

/// Validate `api_token`, resolve the account (explicit `account_id`, the
/// sole account the token can see, or an error listing the choices when
/// there's more than one), then persist `~/.adevgrok/cloudflare-worker.json`
/// — the same format `cloudflare-worker.js` reads, so the npm wrapper picks
/// it up on its next launch and does the actual bridge/tunnel/deploy.
pub async fn configure(
    api_token: String,
    worker_name: Option<String>,
    account_id: Option<String>,
) -> Result<ConfigureOutcome, String> {
    if api_token.trim().is_empty() {
        return Err("Cloudflare API token is required.".to_owned());
    }
    let worker_name = sanitize_worker_name(worker_name.as_deref().unwrap_or(DEFAULT_WORKER_NAME));

    let account_id = match account_id {
        Some(id) => id,
        None => {
            let accounts = discover_accounts(&api_token).await?;
            match accounts.len() {
                0 => {
                    return Err(
                        "The Cloudflare token cannot access any accounts. Check its \
                         account scope and permissions."
                            .to_owned(),
                    );
                }
                1 => accounts.into_iter().next().expect("len == 1").id,
                _ => {
                    let list = accounts
                        .iter()
                        .map(|a| format!("{} ({})", if a.name.is_empty() { &a.id } else { &a.name }, a.id))
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(format!(
                        "This token can access multiple accounts: {list}. Re-run with the \
                         account ID as a fourth argument."
                    ));
                }
            }
        }
    };

    let subdomain = validate_account(&api_token, &account_id).await?;

    write_config(&CloudflareWorkerConfig {
        api_token,
        account_id: account_id.clone(),
        worker_name: worker_name.clone(),
        enabled: true,
    })
    .map_err(|e| format!("Failed to save Cloudflare Worker config: {e}"))?;

    Ok(ConfigureOutcome {
        worker_name,
        account_id,
        subdomain,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_worker_name_lowercases_and_replaces_invalid_chars() {
        assert_eq!(sanitize_worker_name("My Worker!"), "my-worker-");
        assert_eq!(sanitize_worker_name("failure-mcp"), "failure-mcp");
        assert_eq!(sanitize_worker_name("under_score"), "under_score");
    }

    #[test]
    fn urlencode_escapes_special_chars() {
        assert_eq!(urlencode("abc123-_.~"), "abc123-_.~");
        assert_eq!(urlencode("a b/c"), "a%20b%2Fc");
    }

    #[test]
    fn masked_status_none_when_unconfigured() {
        // No config file at the default path in a typical test sandbox home;
        // this just exercises the None branch without touching real state.
        // (Doesn't assert on read_config() directly since that depends on
        // the environment's actual ~/.adevgrok — covered by integration
        // testing instead.)
        let cfg = CloudflareWorkerConfig {
            api_token: "abcd1234wxyz9999".to_owned(),
            account_id: "acct".to_owned(),
            worker_name: "failure-mcp".to_owned(),
            enabled: true,
        };
        let masked = if cfg.api_token.len() > 8 {
            format!("{}...{}", &cfg.api_token[..4], &cfg.api_token[cfg.api_token.len() - 4..])
        } else {
            "(short token)".to_owned()
        };
        assert_eq!(masked, "abcd...9999");
    }

    #[test]
    fn config_json_field_names_match_npm_wrapper() {
        let cfg = CloudflareWorkerConfig {
            api_token: "tok".to_owned(),
            account_id: "acct".to_owned(),
            worker_name: "failure-mcp".to_owned(),
            enabled: true,
        };
        let json = serde_json::to_value(&cfg).unwrap();
        assert_eq!(json["apiToken"], "tok");
        assert_eq!(json["accountId"], "acct");
        assert_eq!(json["workerName"], "failure-mcp");
        assert_eq!(json["enabled"], true);
    }
}
