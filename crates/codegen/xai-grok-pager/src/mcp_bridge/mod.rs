//! Native remote MCP bridge (`/mcp start|status|stop`) — an in-process
//! Streamable HTTP MCP server exposing Failure's session lifecycle as MCP
//! tools, without needing Node/npm at all. See
//! `npm/failure/bin/mcp-server.js` for the separate, Node-based bridge the
//! npm package ships — this is a from-scratch, native equivalent, not a
//! wrapper around it.

mod agent;
mod http;
mod server_handler;
mod state_file;
mod tunnel;

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use parking_lot::Mutex;
use rand::RngCore;

use agent::BridgeAgent;

const DEFAULT_PORT: u16 = 2420;

struct RunningBridge {
    handle: http::ServerHandle,
    agent: Arc<BridgeAgent>,
    _tunnel: Option<tunnel::Tunnel>,
    port: u16,
    token: String,
    public_url: Option<String>,
}

static BRIDGE: OnceLock<Mutex<Option<RunningBridge>>> = OnceLock::new();

fn slot() -> &'static Mutex<Option<RunningBridge>> {
    BRIDGE.get_or_init(|| Mutex::new(None))
}

#[derive(Debug, Clone)]
pub struct BridgeStatus {
    pub running: bool,
    pub local_url: Option<String>,
    pub public_url: Option<String>,
}

/// Start the bridge. No-ops (returns the existing status) if one is already
/// running in this process — `/mcp start` twice in a row is not an error.
pub async fn start(
    port: Option<u16>,
    token: Option<String>,
    cwd: PathBuf,
) -> anyhow::Result<BridgeStatus> {
    if let Some(running) = slot().lock().as_ref() {
        return Ok(status_of(running));
    }

    let port = port.unwrap_or(DEFAULT_PORT);
    let token = token.unwrap_or_else(generate_token);

    let agent = BridgeAgent::spawn().await?;
    let handle = http::serve(port, token.clone(), agent.clone(), cwd).await?;
    // Report the port actually bound, not the one requested — they differ
    // when the user asks for an ephemeral port (`/mcp start 0`).
    let port = handle.addr.port();

    let (tunnel_handle, tunnel_base_url) = match tunnel::start(port, Duration::from_secs(10)).await
    {
        Ok(Some((t, url))) => (Some(t), Some(url)),
        Ok(None) => (None, None),
        Err(e) => {
            tracing::warn!("MCP bridge: tunnel unavailable: {e}");
            (None, None)
        }
    };
    let public_url = tunnel_base_url.map(|u| format!("{u}/mcp?token={token}"));

    let local_url = format!("http://127.0.0.1:{port}/mcp?token={token}");
    let _ = state_file::write(&state_file::McpState {
        pid: std::process::id(),
        token: token.clone(),
        local_url,
        public_url: public_url.clone(),
        started_at: chrono::Utc::now().to_rfc3339(),
    });

    let running = RunningBridge {
        handle,
        agent,
        _tunnel: tunnel_handle,
        port,
        token,
        public_url,
    };
    let reported = status_of(&running);
    *slot().lock() = Some(running);
    Ok(reported)
}

fn status_of(running: &RunningBridge) -> BridgeStatus {
    BridgeStatus {
        running: true,
        local_url: Some(format!(
            "http://127.0.0.1:{}/mcp?token={}",
            running.port, running.token
        )),
        public_url: running.public_url.clone(),
    }
}

/// Current bridge status, from this process's own running instance (not a
/// disk read — a bridge started by a different process won't show here;
/// use [`external_state`] for that).
pub fn status() -> BridgeStatus {
    match slot().lock().as_ref() {
        Some(running) => status_of(running),
        None => BridgeStatus {
            running: false,
            local_url: None,
            public_url: None,
        },
    }
}

/// `~/.adevgrok/mcp.json` written by a bridge in a *different* process (this
/// one, an earlier run, or the npm wrapper's Node bridge). Returns `None`
/// when the file is missing/unreadable or was written by this process (the
/// in-process [`status`] already covers that case authoritatively).
pub fn external_state() -> Option<state_file::McpState> {
    let state = state_file::read()?;
    (state.pid != std::process::id()).then_some(state)
}

/// Stop the bridge. Returns `true` if one was actually running.
pub fn stop() -> bool {
    let Some(running) = slot().lock().take() else {
        return false;
    };
    running.handle.shutdown();
    // Cancel the agent explicitly rather than relying on the last Arc drop:
    // the HTTP service factory closure holds its own clone, and its teardown
    // timing after graceful shutdown shouldn't decide when the agent dies.
    running.agent.shutdown();
    state_file::remove();
    true
}

fn generate_token() -> String {
    use base64::Engine;
    let mut bytes = [0u8; 24];
    rand::rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
