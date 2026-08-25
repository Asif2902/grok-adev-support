//! Dedicated in-process agent instance for the native MCP bridge (`/mcp start`).
//!
//! Spawns its own `MvpAgent` via `spawn_grok_shell`, completely independent
//! from whatever the interactive TUI is doing with its own session — so
//! draining this agent's notification stream never competes with the TUI's
//! own exclusive `channel.rx` consumer (see `app/acp_handler/mod.rs`).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use agent_client_protocol as acp;
use anyhow::{Context, Result};
use parking_lot::Mutex;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use xai_acp_lib::{AcpAgentTx, acp_send};
use xai_grok_shell::util::config as cli_config;

/// One session's streamed `AgentMessageChunk` text, forwarded here while a
/// `failure_send_message` call for that session is in flight. Removed once
/// the call completes (see `BridgeAgent::send_message`).
type ChunkRegistry = Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>;

/// A dedicated agent instance backing the MCP bridge. One per running
/// bridge; created fresh on `/mcp start`, torn down on `/mcp stop`.
pub struct BridgeAgent {
    tx: AcpAgentTx,
    chunk_txs: ChunkRegistry,
    cancel: CancellationToken,
}

impl Drop for BridgeAgent {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

impl BridgeAgent {
    /// Spawn a fresh agent and connect, reusing the exact same
    /// spawn/initialize/authenticate sequence the interactive TUI uses
    /// (`crate::acp::connect`) — including whatever credentials are already
    /// on disk. Never falls back to an interactive login flow: there's no
    /// human on the other end of a remote MCP call, so a session that would
    /// need one is a hard error instead.
    pub async fn spawn() -> Result<Arc<Self>> {
        let cancel = CancellationToken::new();
        let conn = crate::acp::connect(
            &cancel,
            crate::acp::ConnectFlags {
                // The bridge has no human to answer permission prompts —
                // every session it opens must run always-approve, matching
                // the npm bridge's `adevgrok agent --always-approve stdio`.
                default_yolo_mode: true,
                ..Default::default()
            },
        )
        .await
        .context("MCP bridge: failed to start the agent")?;

        if conn.needs_login {
            anyhow::bail!(
                "MCP bridge: not signed in. Run `failure` interactively and sign in \
                 (or set an API key) before starting the bridge."
            );
        }

        let tx = conn.tx;
        let mut rx = conn.rx;
        let chunk_txs: ChunkRegistry = Arc::new(Mutex::new(HashMap::new()));
        let pump_chunk_txs = chunk_txs.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                handle_bridge_acp_message(msg.boxed(), &pump_chunk_txs);
            }
        });

        Ok(Arc::new(Self {
            tx,
            chunk_txs,
            cancel: conn.cancel,
        }))
    }

    /// `failure_new_chat`: create a new session in `cwd`.
    pub async fn new_chat(&self, cwd: &std::path::Path) -> Result<acp::NewSessionResponse> {
        let mcp_servers = cli_config::load_mcp_servers(
            cwd,
            &xai_grok_tools::types::compat::CompatConfig::default(),
        );
        Ok(acp_send(
            acp::NewSessionRequest::new(cwd.to_path_buf()).mcp_servers(mcp_servers),
            &self.tx,
        )
        .await?)
    }

    /// `failure_continue_chat`: load an existing session by id.
    pub async fn continue_chat(
        &self,
        session_id: &str,
        cwd: &std::path::Path,
    ) -> Result<acp::LoadSessionResponse> {
        let mcp_servers = cli_config::load_mcp_servers(
            cwd,
            &xai_grok_tools::types::compat::CompatConfig::default(),
        );
        Ok(acp_send(
            acp::LoadSessionRequest::new(acp::SessionId::new(session_id.to_string()), cwd.to_path_buf())
                .mcp_servers(mcp_servers),
            &self.tx,
        )
        .await?)
    }

    /// `failure_send_message`: prompt a session and collect the final
    /// assistant text, accumulated from streamed `session/update` chunks
    /// while `session/prompt`'s own response only carries the stop reason.
    pub async fn send_message(
        &self,
        session_id: &str,
        message: &str,
        timeout: Duration,
    ) -> Result<(acp::PromptResponse, String)> {
        let (chunk_tx, mut chunk_rx) = mpsc::unbounded_channel::<String>();
        self.chunk_txs
            .lock()
            .insert(session_id.to_string(), chunk_tx);

        // Always deregister, even on error/timeout, so a stale sender never
        // lingers in the registry after this call returns.
        struct Deregister<'a> {
            chunk_txs: &'a ChunkRegistry,
            session_id: &'a str,
        }
        impl Drop for Deregister<'_> {
            fn drop(&mut self) {
                self.chunk_txs.lock().remove(self.session_id);
            }
        }
        let _deregister = Deregister {
            chunk_txs: &self.chunk_txs,
            session_id,
        };

        let request = acp::PromptRequest::new(
            acp::SessionId::new(session_id.to_string()),
            vec![acp::ContentBlock::from(message.to_string())],
        );
        let mut prompt_fut = Box::pin(acp_send(request, &self.tx));
        let mut text = String::new();

        let result = tokio::time::timeout(timeout, async {
            loop {
                tokio::select! {
                    biased;
                    chunk = chunk_rx.recv() => {
                        match chunk {
                            Some(t) => text.push_str(&t),
                            None => {}
                        }
                    }
                    res = &mut prompt_fut => {
                        return res;
                    }
                }
            }
        })
        .await;

        // Drain any trailing chunks buffered after the prompt resolved but
        // before this call returns (mirrors headless's grace-period drain).
        while let Ok(t) = chunk_rx.try_recv() {
            text.push_str(&t);
        }

        let response = result
            .map_err(|_| anyhow::anyhow!("MCP bridge: prompt timed out after {timeout:?}"))??;
        Ok((response, text))
    }

    /// `failure_rpc` (scoped): any `x.ai/*` extension method, forwarded
    /// as-is. Core session methods (`session/new`, `session/prompt`, ...)
    /// go through the dedicated typed tools above instead — the in-process
    /// bridge talks typed ACP requests, not raw JSON-RPC over stdio like the
    /// npm bridge, so a fully generic passthrough isn't available here.
    pub async fn ext_rpc(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let request = acp::ExtRequest::new(
            method.to_string(),
            serde_json::value::to_raw_value(&params)
                .context("MCP bridge: serialize ext_rpc params")?
                .into(),
        );
        let resp: acp::ExtResponse = acp_send(request, &self.tx).await?;
        Ok(serde_json::from_str(resp.0.get()).unwrap_or(serde_json::Value::Null))
    }

    pub fn is_running(&self) -> bool {
        !self.cancel.is_cancelled()
    }

    pub fn shutdown(&self) {
        self.cancel.cancel();
    }
}

/// Handle one message from the bridge agent's own `channel.rx`. Every
/// permission request is auto-approved (the agent runs `default_yolo_mode`,
/// but a request can still be emitted and MUST be answered or the agent
/// hangs waiting on it); every notification is acked; streamed text chunks
/// are forwarded to whichever `send_message` call is listening for that
/// session, if any.
fn handle_bridge_acp_message(msg: xai_acp_lib::AcpClientMessageBox, chunk_txs: &ChunkRegistry) {
    use xai_acp_lib::AcpClientMessageBox;
    match msg {
        AcpClientMessageBox::SessionNotification(boxed) => {
            if let acp::SessionUpdate::AgentMessageChunk(chunk) = &boxed.request.update
                && let acp::ContentBlock::Text(text) = &chunk.content
                && !text.text.is_empty()
            {
                let txs = chunk_txs.lock();
                if let Some(tx) = txs.get(boxed.request.session_id.0.as_ref()) {
                    let _ = tx.send(text.text.clone());
                }
            }
            let _ = boxed.response_tx.send(Ok(()));
        }
        AcpClientMessageBox::RequestPermission(req) => {
            let resp = auto_respond_to_permissions(&req.request).unwrap_or_else(|| {
                acp::RequestPermissionResponse::new(acp::RequestPermissionOutcome::Cancelled)
            });
            let _ = req.response_tx.send(Ok(resp));
        }
        AcpClientMessageBox::ExtNotification(notif) => {
            let _ = notif.response_tx.send(Ok(()));
        }
        AcpClientMessageBox::WaitForTerminalExit(args) => {
            let _ = args
                .response_tx
                .send(Err(crate::acp::wait_for_exit_not_supported("MCP bridge")));
        }
        _ => {}
    }
}

fn auto_respond_to_permissions(
    args: &acp::RequestPermissionRequest,
) -> Option<acp::RequestPermissionResponse> {
    for &option_kind in &[
        acp::PermissionOptionKind::AllowOnce,
        acp::PermissionOptionKind::AllowAlways,
    ] {
        for option in &args.options {
            if option.kind == option_kind {
                return Some(acp::RequestPermissionResponse::new(
                    acp::RequestPermissionOutcome::Selected(acp::SelectedPermissionOutcome::new(
                        option.option_id.clone(),
                    )),
                ));
            }
        }
    }
    None
}
