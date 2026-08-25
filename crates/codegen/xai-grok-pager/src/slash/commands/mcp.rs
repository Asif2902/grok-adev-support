//! `/mcp start [port] [token]` / `/mcp status` / `/mcp stop` — the native,
//! in-process remote MCP bridge. No Node/npm required, unlike the npm
//! package's separate bridge (`npm/failure/bin/mcp-server.js`).

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};

pub struct McpCommand;

impl SlashCommand for McpCommand {
    fn name(&self) -> &str {
        "mcp"
    }

    fn description(&self) -> &str {
        "Start/stop the native remote MCP bridge (no Node/npm required)"
    }

    fn usage(&self) -> &str {
        "/mcp start [port] [token] | /mcp status | /mcp stop"
    }

    fn takes_args(&self) -> bool {
        true
    }

    fn args_required(&self) -> bool {
        true
    }

    fn arg_placeholder(&self) -> Option<&str> {
        Some("start|status|stop")
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        let mut tokens = args.split_whitespace();
        let Some(sub) = tokens.next() else {
            return CommandResult::Error(format!("Usage: {}", self.usage()));
        };

        match sub.to_ascii_lowercase().as_str() {
            "start" => {
                let port = match tokens.next() {
                    Some(raw) => match raw.parse::<u16>() {
                        Ok(p) => Some(p),
                        Err(_) => {
                            return CommandResult::Error(format!(
                                "Invalid port '{raw}'. Usage: {}",
                                self.usage()
                            ));
                        }
                    },
                    None => None,
                };
                let token = tokens
                    .next()
                    .map(str::to_owned)
                    .or_else(|| std::env::var("FAILURE_MCP_TOKEN").ok());
                CommandResult::Action(Action::McpBridgeStart { port, token })
            }
            "status" => CommandResult::Message(format_status(&crate::mcp_bridge::status())),
            "stop" => {
                if crate::mcp_bridge::stop() {
                    CommandResult::Message("MCP bridge stopped.".to_owned())
                } else {
                    CommandResult::Message("MCP bridge is not running.".to_owned())
                }
            }
            other => CommandResult::Error(format!(
                "Unknown /mcp subcommand '{other}'. Usage: {}",
                self.usage()
            )),
        }
    }
}

fn format_status(status: &crate::mcp_bridge::BridgeStatus) -> String {
    if !status.running {
        // Not running in this process — but another process (an earlier run,
        // or the npm wrapper's Node bridge) may have left a live one behind.
        if let Some(external) = crate::mcp_bridge::external_state() {
            return format!(
                "No MCP bridge running in this process, but ~/.adevgrok/mcp.json \
                 reports one from pid {} (started {}).\nLocal:  {}{}",
                external.pid,
                external.started_at,
                external.local_url,
                external
                    .public_url
                    .as_deref()
                    .map(|u| format!("\nPublic: {u}"))
                    .unwrap_or_default(),
            );
        }
        return "MCP bridge is not running. Start it with `/mcp start`.".to_owned();
    }
    let mut msg = format!(
        "MCP bridge running.\nLocal:  {}",
        status.local_url.as_deref().unwrap_or("?")
    );
    match &status.public_url {
        Some(public) => msg.push_str(&format!("\nPublic: {public}")),
        None => msg.push_str(
            "\nNo public URL (install `cloudflared` for one, or `/mcp-worker configure` for a stable URL).",
        ),
    }
    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ScreenMode;
    use crate::app::bundle::BundleState;
    use crate::settings::PagerLocalSnapshot;

    static EMPTY_BUNDLE: BundleState = BundleState {
        has_cache: false,
        version: String::new(),
        personas: Vec::new(),
        roles: Vec::new(),
        agents: Vec::new(),
        skills: Vec::new(),
        persona_details: Vec::new(),
        role_details: Vec::new(),
    };

    fn dummy_ctx() -> CommandExecCtx<'static> {
        CommandExecCtx {
            models: Box::leak(Box::new(crate::acp::model_state::ModelState::default())),
            session_id: None,
            bundle_state: &EMPTY_BUNDLE,
            screen_mode: ScreenMode::Inline,
            pager_state: PagerLocalSnapshot {
                multiline_mode: false,
                yolo_mode: false,
                ..PagerLocalSnapshot::default()
            },
        }
    }

    #[test]
    fn missing_subcommand_errors() {
        let mut ctx = dummy_ctx();
        assert!(matches!(McpCommand.run(&mut ctx, ""), CommandResult::Error(_)));
    }

    #[test]
    fn unknown_subcommand_errors() {
        let mut ctx = dummy_ctx();
        assert!(matches!(
            McpCommand.run(&mut ctx, "launch"),
            CommandResult::Error(_)
        ));
    }

    #[test]
    fn start_with_no_args_dispatches_action_with_defaults() {
        let mut ctx = dummy_ctx();
        // SAFETY-adjacent: clear any test-order-dependent env leakage.
        unsafe { std::env::remove_var("FAILURE_MCP_TOKEN") };
        match McpCommand.run(&mut ctx, "start") {
            CommandResult::Action(Action::McpBridgeStart { port, token }) => {
                assert_eq!(port, None);
                assert_eq!(token, None);
            }
            other => panic!("expected McpBridgeStart action, got {other:?}"),
        }
    }

    #[test]
    fn start_with_port_and_token_dispatches_action() {
        let mut ctx = dummy_ctx();
        match McpCommand.run(&mut ctx, "start 8080 my-secret") {
            CommandResult::Action(Action::McpBridgeStart { port, token }) => {
                assert_eq!(port, Some(8080));
                assert_eq!(token.as_deref(), Some("my-secret"));
            }
            other => panic!("expected McpBridgeStart action, got {other:?}"),
        }
    }

    #[test]
    fn start_with_invalid_port_errors() {
        let mut ctx = dummy_ctx();
        assert!(matches!(
            McpCommand.run(&mut ctx, "start notaport"),
            CommandResult::Error(_)
        ));
    }

    #[test]
    fn status_returns_message() {
        let mut ctx = dummy_ctx();
        assert!(matches!(
            McpCommand.run(&mut ctx, "status"),
            CommandResult::Message(_)
        ));
    }

    #[test]
    fn stop_returns_message() {
        let mut ctx = dummy_ctx();
        assert!(matches!(
            McpCommand.run(&mut ctx, "stop"),
            CommandResult::Message(_)
        ));
    }
}
