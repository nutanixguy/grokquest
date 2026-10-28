use std::env;

use crate::ai_request::AiRequest;
use crate::game_engine::GameEngine;
use crate::llm::{choose_setting, provider_from_selection, resolve_api_key, select_provider};
use clap::Parser;

mod ai_request;
mod game_engine;
mod game_prompt;
mod game_state;
mod gpt_model;
mod llm;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// LLM backend: `openai` (default), `xai`, or `compatible`.
    ///
    /// When omitted, `AI_PROVIDER` is used, then `LLM_PROVIDER`.
    #[arg(long)]
    provider: Option<String>,

    /// OpenAI API key. Required when the provider is openai.
    #[arg(long, env = "OPENAI_API_KEY")]
    api_key: Option<String>,

    /// xAI API key. Required when the provider is xai.
    #[arg(long, env = "XAI_API_KEY")]
    xai_api_key: Option<String>,

    /// API key for an OpenAI-compatible endpoint. Required when the provider is compatible.
    #[arg(long, env = "AI_API_KEY")]
    ai_api_key: Option<String>,

    /// Model id. Defaults to `gpt-4o-mini` for OpenAI and `grok-4.7` for xAI.
    ///
    /// When omitted, `AI_MODEL` is used, then `LLM_MODEL`. Required for `compatible`.
    #[arg(long)]
    model: Option<String>,

    /// API root, such as `https://api.example.com/v1`.
    ///
    /// When omitted, `AI_BASE_URL` is used. Required for `compatible`.
    #[arg(long)]
    base_url: Option<String>,

    /// Reasoning effort for xAI: `low` (default) or `medium`.
    #[arg(long, env = "LLM_REASONING_EFFORT")]
    reasoning_effort: Option<String>,
}

fn env_nonempty(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let ai_provider = env_nonempty("AI_PROVIDER");
    let llm_provider = env_nonempty("LLM_PROVIDER");
    let ai_model = env_nonempty("AI_MODEL");
    let llm_model = env_nonempty("LLM_MODEL");
    let ai_base_url = env_nonempty("AI_BASE_URL");

    let provider = choose_setting(
        args.provider.as_deref(),
        ai_provider.as_deref(),
        llm_provider.as_deref(),
    );
    let model = choose_setting(
        args.model.as_deref(),
        ai_model.as_deref(),
        llm_model.as_deref(),
    );
    let base_url = choose_setting(args.base_url.as_deref(), ai_base_url.as_deref(), None);

    let selection = select_provider(provider, model, args.reasoning_effort.as_deref(), base_url)?;
    let api_key = resolve_api_key(
        selection.kind,
        args.api_key.as_deref(),
        args.xai_api_key.as_deref(),
        args.ai_api_key.as_deref(),
    )?;
    let ai = AiRequest::new(provider_from_selection(selection, api_key));
    ai.check_api_key().await?;
    let mut game = GameEngine::new(ai);
    game.start_game().await?;

    Ok(())
}
