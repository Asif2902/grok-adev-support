//! `/mcp-worker configure <token> [worker-name] [account-id]` — save
//! Cloudflare Worker credentials for the stable remote-MCP URL feature.
//!
//! Mirrors `adevgrok mcp-worker configure`/`status`/`disable` (the npm
//! package's `bin/cloudflare-worker.js`), but only handles credential
//! storage: validating the token and writing
//! `~/.adevgrok/cloudflare-worker.json`. The actual local MCP bridge,
//! Cloudflare Quick Tunnel, and Worker deploy are npm-wrapper-only —
//! launching via the npm package's `failure` command picks up what's
//! saved here on its next start.

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};

pub struct McpWorkerCommand;

impl SlashCommand for McpWorkerCommand {
    fn name(&self) -> &str {
        "mcp-worker"
    }

    fn description(&self) -> &str {
        "Configure a stable Cloudflare Worker URL for remote MCP access (npm package only)"
    }

    fn usage(&self) -> &str {
        "/mcp-worker configure <cloudflare-api-token> [worker-name] [account-id]"
    }

    fn takes_args(&self) -> bool {
        true
    }

    fn args_required(&self) -> bool {
        true
    }

    fn arg_placeholder(&self) -> Option<&str> {
        Some("configure <token> [worker-name] [account-id]")
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        let mut tokens = args.split_whitespace();
        let Some(sub) = tokens.next() else {
            return CommandResult::Error(format!("Usage: {}", self.usage()));
        };

        match sub.to_ascii_lowercase().as_str() {
            "configure" => {
                let Some(api_token) = tokens.next() else {
                    return CommandResult::Error(format!(
                        "Missing Cloudflare API token. Usage: {}",
                        self.usage()
                    ));
                };
                let worker_name = tokens.next();
                let account_id = tokens.next();
                CommandResult::Action(Action::ConfigureMcpWorker {
                    api_token: api_token.to_owned(),
                    worker_name: worker_name.map(str::to_owned),
                    account_id: account_id.map(str::to_owned),
                })
            }
            "status" => match xai_grok_shell::cloudflare_worker::masked_status() {
                Some(status) => CommandResult::Message(status),
                None => CommandResult::Message("Cloudflare Worker access is not configured.".to_owned()),
            },
            "disable" => {
                xai_grok_shell::cloudflare_worker::remove_config();
                CommandResult::Message(
                    "Cloudflare Worker access disabled and local credentials removed.".to_owned(),
                )
            }
            other => CommandResult::Error(format!(
                "Unknown /mcp-worker subcommand '{other}'. Usage: {}",
                self.usage()
            )),
        }
    }
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
        assert!(matches!(
            McpWorkerCommand.run(&mut ctx, ""),
            CommandResult::Error(_)
        ));
    }

    #[test]
    fn unknown_subcommand_errors() {
        let mut ctx = dummy_ctx();
        assert!(matches!(
            McpWorkerCommand.run(&mut ctx, "deploy"),
            CommandResult::Error(_)
        ));
    }

    #[test]
    fn configure_missing_token_errors() {
        let mut ctx = dummy_ctx();
        assert!(matches!(
            McpWorkerCommand.run(&mut ctx, "configure"),
            CommandResult::Error(_)
        ));
    }

    #[test]
    fn configure_with_token_only_dispatches_action() {
        let mut ctx = dummy_ctx();
        let result = McpWorkerCommand.run(&mut ctx, "configure cf-token-123");
        match result {
            CommandResult::Action(Action::ConfigureMcpWorker {
                api_token,
                worker_name,
                account_id,
            }) => {
                assert_eq!(api_token, "cf-token-123");
                assert_eq!(worker_name, None);
                assert_eq!(account_id, None);
            }
            other => panic!("expected ConfigureMcpWorker action, got {other:?}"),
        }
    }

    #[test]
    fn configure_with_all_args_dispatches_action() {
        let mut ctx = dummy_ctx();
        let result = McpWorkerCommand.run(&mut ctx, "configure cf-token-123 my-worker acct-id");
        match result {
            CommandResult::Action(Action::ConfigureMcpWorker {
                api_token,
                worker_name,
                account_id,
            }) => {
                assert_eq!(api_token, "cf-token-123");
                assert_eq!(worker_name.as_deref(), Some("my-worker"));
                assert_eq!(account_id.as_deref(), Some("acct-id"));
            }
            other => panic!("expected ConfigureMcpWorker action, got {other:?}"),
        }
    }

    #[test]
    fn disable_returns_message() {
        let mut ctx = dummy_ctx();
        assert!(matches!(
            McpWorkerCommand.run(&mut ctx, "disable"),
            CommandResult::Message(_)
        ));
    }
}
