//! `/provider add <name> <api-key> [base-url]` — configure a custom
//! (BYOP) provider without hand-editing `config.toml`.
//!
//! Also: `/provider setup` opens the first-launch-style provider picker
//! (welcome or in-session) so users don't have to remember preset names.
//!
//! Persists `[provider.<name>]`/`[model.<name>]` and stores the API key via
//! the same provider-scoped secret storage `adevgrok login --provider` uses.
//! The existing config-file watcher picks up the change and refreshes the
//! model catalog automatically — no restart needed.

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};
use xai_grok_shell::agent::config::{builtin_provider_presets, is_builtin_provider_preset};

pub struct ProviderCommand;

impl SlashCommand for ProviderCommand {
    fn name(&self) -> &str {
        "provider"
    }

    fn description(&self) -> &str {
        "Configure a custom AI provider (OpenAI, Anthropic, OpenRouter, Groq, …) or run /provider setup"
    }

    fn usage(&self) -> &str {
        "/provider add <name> <api-key> [base-url]  |  /provider setup"
    }

    fn takes_args(&self) -> bool {
        true
    }

    fn args_required(&self) -> bool {
        true
    }

    fn arg_placeholder(&self) -> Option<&str> {
        Some("add <name> <api-key> [base-url] | setup")
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        let mut tokens = args.split_whitespace();
        let Some(sub) = tokens.next() else {
            return CommandResult::Error(format!("Usage: {}", self.usage()));
        };
        if sub.eq_ignore_ascii_case("setup") {
            return CommandResult::Action(Action::StartByopSetup);
        }
        if !sub.eq_ignore_ascii_case("add") {
            return CommandResult::Error(format!(
                "Unknown /provider subcommand '{sub}'. Usage: {}",
                self.usage()
            ));
        }

        let Some(name) = tokens.next() else {
            return CommandResult::Error(format!(
                "Missing provider name. Usage: {}",
                self.usage()
            ));
        };
        let Some(api_key) = tokens.next() else {
            return CommandResult::Error(format!(
                "Missing API key. Usage: {}",
                self.usage()
            ));
        };
        let base_url = tokens.next();

        if base_url.is_none() && !is_builtin_provider_preset(name) {
            let known = builtin_provider_presets()
                .iter()
                .map(|p| p.id)
                .collect::<Vec<_>>()
                .join("/");
            return CommandResult::Error(format!(
                "'{name}' isn't a built-in provider ({known}), so a base URL is required. \
                 Usage: {}",
                self.usage()
            ));
        }

        CommandResult::Action(Action::AddProvider {
            name: name.to_owned(),
            api_key: api_key.to_owned(),
            base_url: base_url.map(str::to_owned),
            complete_auth: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::bundle::BundleState;
    use crate::app::ScreenMode;
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
            ProviderCommand.run(&mut ctx, ""),
            CommandResult::Error(_)
        ));
    }

    #[test]
    fn unknown_subcommand_errors() {
        let mut ctx = dummy_ctx();
        assert!(matches!(
            ProviderCommand.run(&mut ctx, "remove openai"),
            CommandResult::Error(_)
        ));
    }

    #[test]
    fn setup_subcommand_opens_wizard() {
        let mut ctx = dummy_ctx();
        assert!(matches!(
            ProviderCommand.run(&mut ctx, "setup"),
            CommandResult::Action(Action::StartByopSetup)
        ));
    }

    #[test]
    fn known_preset_without_base_url_is_ok() {
        let mut ctx = dummy_ctx();
        let result = ProviderCommand.run(&mut ctx, "add openai sk-test123");
        match result {
            CommandResult::Action(Action::AddProvider {
                name,
                api_key,
                base_url,
                complete_auth,
            }) => {
                assert_eq!(name, "openai");
                assert_eq!(api_key, "sk-test123");
                assert_eq!(base_url, None);
                assert!(!complete_auth);
            }
            other => panic!("expected AddProvider action, got {other:?}"),
        }
    }

    #[test]
    fn openrouter_preset_without_base_url_is_ok() {
        let mut ctx = dummy_ctx();
        let result = ProviderCommand.run(&mut ctx, "add openrouter sk-or-test");
        assert!(matches!(
            result,
            CommandResult::Action(Action::AddProvider { name, .. }) if name == "openrouter"
        ));
    }

    #[test]
    fn custom_name_without_base_url_errors() {
        let mut ctx = dummy_ctx();
        let result = ProviderCommand.run(&mut ctx, "add my-custom-thing sk-test123");
        assert!(matches!(result, CommandResult::Error(_)));
    }

    #[test]
    fn custom_name_with_base_url_is_ok() {
        let mut ctx = dummy_ctx();
        let result = ProviderCommand.run(
            &mut ctx,
            "add zandy-worker orch_test_abc https://ai-orchestrator-worker.example.dev/v1",
        );
        match result {
            CommandResult::Action(Action::AddProvider {
                name,
                api_key,
                base_url,
                ..
            }) => {
                assert_eq!(name, "zandy-worker");
                assert_eq!(api_key, "orch_test_abc");
                assert_eq!(
                    base_url.as_deref(),
                    Some("https://ai-orchestrator-worker.example.dev/v1")
                );
            }
            other => panic!("expected AddProvider action, got {other:?}"),
        }
    }
}
