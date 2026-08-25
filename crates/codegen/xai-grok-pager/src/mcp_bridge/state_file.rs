//! `~/.adevgrok/mcp.json` — matches the npm bridge's JSON shape exactly
//! (`bin/mcp-server.js`'s `saveState`), so either implementation can write
//! it and any reader (Claude, `adevgrok mcp-worker status`, etc.) sees the
//! same shape regardless of which one is running.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpState {
    pub pid: u32,
    pub token: String,
    #[serde(rename = "localUrl")]
    pub local_url: String,
    #[serde(rename = "publicUrl")]
    pub public_url: Option<String>,
    #[serde(rename = "startedAt")]
    pub started_at: String,
}

fn state_path() -> PathBuf {
    xai_grok_shell::util::grok_home::grok_home().join("mcp.json")
}

pub fn write(state: &McpState) -> anyhow::Result<()> {
    let path = state_path();
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(&path, &json)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

pub fn read() -> Option<McpState> {
    let data = std::fs::read_to_string(state_path()).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn remove() {
    let _ = std::fs::remove_file(state_path());
}
