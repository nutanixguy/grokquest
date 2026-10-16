use crate::ai_request::AiRequest;
use crate::game_engine::GameEngine;
use crate::llm::{provider_from_selection, resolve_api_key, select_provider};
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
    /// LLM backend: `openai` (default) or `xai`.
    #[arg(long, env = "LLM_PROVIDER", default_value = "openai")]
    provider: String,

    /// OpenAI API key. Required when the provider is openai.
    #[arg(long, env = "OPENAI_API_KEY")]
    api_key: Option<String>,

    /// xAI API key. Required when the provider is xai.
    #[arg(long, env = "XAI_API_KEY")]
    xai_api_key: Option<String>,

    /// Model id. Defaults to `gpt-4o-mini` for OpenAI and `grok-4.7` for xAI.
    #[arg(long, env = "LLM_MODEL")]
    model: Option<String>,

    /// Reasoning effort for xAI: `low` (default) or `medium`.
    #[arg(long, env = "LLM_REASONING_EFFORT")]
    reasoning_effort: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let selection = select_provider(
        Some(&args.provider),
        args.model.as_deref(),
        args.reasoning_effort.as_deref(),
    )?;
    let api_key = resolve_api_key(
        selection.kind,
        args.api_key.as_deref(),
        args.xai_api_key.as_deref(),
    )?;
    let ai = AiRequest::new(provider_from_selection(selection, api_key));
    ai.check_api_key().await?;
    let mut game = GameEngine::new(ai);
    game.start_game().await?;

    Ok(())
}
