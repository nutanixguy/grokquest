use serde::{Deserialize, Serialize};

use crate::game_prompt::GamePrompt;
use crate::game_state::GameState;
use crate::llm::{ChatClient, ChatRequest, LlmProvider, Message};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameResponse {
    pub story: String,
    pub new_items: Vec<String>,
    pub location: String,
    pub choices: Vec<String>,
    pub summary: String,
}

pub struct AiRequest {
    client: ChatClient,
}

impl AiRequest {
    pub fn new(provider: Box<dyn LlmProvider>) -> Self {
        Self {
            client: ChatClient::new(provider),
        }
    }

    pub async fn check_api_key(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.client.check_api_key().await
    }

    pub async fn json_chat(
        &self,
        game_state: &GameState,
    ) -> Result<GameResponse, Box<dyn std::error::Error>> {
        let messages = vec![
            Message::system(GamePrompt::get_system_prompt()),
            Message::user(&GamePrompt::build_game_context_prompt(game_state)),
        ];
        let request: ChatRequest = self.client.build_request(messages);
        let response_text = self.client.complete(&request).await?;
        let game_response: GameResponse = serde_json::from_str(&response_text)?;
        Ok(game_response)
    }
}
