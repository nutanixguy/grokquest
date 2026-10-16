//! LLM backends for the storyteller.
//!
//! [`LlmProvider`] sits in front of the chat-completions call. [`OpenAiProvider`]
//! is the original OpenAI client. [`XaiProvider`] talks to xAI's OpenAI-compatible
//! API.

use serde::{Deserialize, Serialize};

use crate::gpt_model::GptModel;

pub const OPENAI_CHAT_COMPLETIONS_URL: &str = "https://api.openai.com/v1/chat/completions";
pub const XAI_CHAT_COMPLETIONS_URL: &str = "https://api.x.ai/v1/chat/completions";
pub const OPENAI_MODELS_URL: &str = "https://api.openai.com/v1/models";
pub const XAI_MODELS_URL: &str = "https://api.x.ai/v1/models";
pub const OPENAI_DEFAULT_MODEL: &str = "gpt-4o-mini";
pub const XAI_DEFAULT_MODEL: &str = "grok-4.7";

const STORY_TOKEN_LIMIT: u32 = 3000;
const STORY_TEMPERATURE: f32 = 0.9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    OpenAi,
    Xai,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Xai => "xai",
        }
    }

    pub fn chat_completions_url(self) -> &'static str {
        match self {
            Self::OpenAi => OPENAI_CHAT_COMPLETIONS_URL,
            Self::Xai => XAI_CHAT_COMPLETIONS_URL,
        }
    }

    pub fn models_url(self) -> &'static str {
        match self {
            Self::OpenAi => OPENAI_MODELS_URL,
            Self::Xai => XAI_MODELS_URL,
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            Self::OpenAi => OPENAI_DEFAULT_MODEL,
            Self::Xai => XAI_DEFAULT_MODEL,
        }
    }

    pub fn api_key_env(self) -> &'static str {
        match self {
            Self::OpenAi => "OPENAI_API_KEY",
            Self::Xai => "XAI_API_KEY",
        }
    }
}

impl std::str::FromStr for ProviderKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "openai" => Ok(Self::OpenAi),
            "xai" => Ok(Self::Xai),
            other => Err(format!(
                "unknown LLM provider '{other}'. Expected 'openai' or 'xai'."
            )),
        }
    }
}

/// How hard an xAI reasoning model thinks before answering.
///
/// The interactive game defaults to [`ReasoningEffort::Low`]. `medium` is the
/// other supported setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Low,
    Medium,
}

impl std::str::FromStr for ReasoningEffort {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            other => Err(format!(
                "unknown reasoning effort '{other}'. Expected 'low' or 'medium'."
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSelection {
    pub kind: ProviderKind,
    pub model: String,
    pub reasoning_effort: Option<ReasoningEffort>,
}

/// Choose a backend from CLI or environment values.
///
/// A missing or blank provider stays on OpenAI, which is the original backend.
/// xAI defaults to `grok-4.7` and `low` reasoning effort.
pub fn select_provider(
    provider: Option<&str>,
    model: Option<&str>,
    reasoning_effort: Option<&str>,
) -> Result<ProviderSelection, String> {
    let kind = match provider.map(str::trim).filter(|value| !value.is_empty()) {
        None => ProviderKind::OpenAi,
        Some(value) => value.parse()?,
    };

    let model = match model.map(str::trim).filter(|value| !value.is_empty()) {
        Some(explicit) => canonicalize_model(kind, explicit)?,
        None => kind.default_model().to_string(),
    };

    let reasoning_effort = match kind {
        ProviderKind::OpenAi => None,
        ProviderKind::Xai => Some(parse_optional_reasoning(reasoning_effort)?),
    };

    Ok(ProviderSelection {
        kind,
        model,
        reasoning_effort,
    })
}

fn parse_optional_reasoning(value: Option<&str>) -> Result<ReasoningEffort, String> {
    match value.map(str::trim).filter(|effort| !effort.is_empty()) {
        None => Ok(ReasoningEffort::Low),
        Some(effort) => effort.parse(),
    }
}

fn canonicalize_model(kind: ProviderKind, model: &str) -> Result<String, String> {
    match kind {
        ProviderKind::OpenAi => {
            let parsed = GptModel::try_from(model)?;
            Ok(parsed.as_str().to_string())
        }
        ProviderKind::Xai => Ok(model.to_string()),
    }
}

pub fn resolve_api_key(
    kind: ProviderKind,
    openai_api_key: Option<&str>,
    xai_api_key: Option<&str>,
) -> Result<String, String> {
    let value = match kind {
        ProviderKind::OpenAi => openai_api_key,
        ProviderKind::Xai => xai_api_key,
    };
    match value.map(str::trim).filter(|key| !key.is_empty()) {
        Some(key) => Ok(key.to_string()),
        None => Err(format!(
            "missing API key for the {} provider. Set {}.",
            kind.as_str(),
            kind.api_key_env()
        )),
    }
}

pub trait LlmProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn model(&self) -> &str;
    fn api_key(&self) -> &str;
    fn reasoning_effort(&self) -> Option<ReasoningEffort>;

    fn chat_completions_url(&self) -> &'static str {
        self.kind().chat_completions_url()
    }

    fn models_url(&self) -> &'static str {
        self.kind().models_url()
    }
}

pub struct OpenAiProvider {
    api_key: String,
    model: String,
}

pub struct XaiProvider {
    api_key: String,
    model: String,
    reasoning_effort: ReasoningEffort,
}

impl LlmProvider for OpenAiProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::OpenAi
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn api_key(&self) -> &str {
        &self.api_key
    }

    fn reasoning_effort(&self) -> Option<ReasoningEffort> {
        None
    }
}

impl LlmProvider for XaiProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Xai
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn api_key(&self) -> &str {
        &self.api_key
    }

    fn reasoning_effort(&self) -> Option<ReasoningEffort> {
        Some(self.reasoning_effort)
    }
}

pub fn provider_from_selection(
    selection: ProviderSelection,
    api_key: String,
) -> Box<dyn LlmProvider> {
    match selection.kind {
        ProviderKind::OpenAi => Box::new(OpenAiProvider {
            api_key,
            model: selection.model,
        }),
        ProviderKind::Xai => Box::new(XaiProvider {
            api_key,
            model: selection.model,
            reasoning_effort: selection.reasoning_effort.unwrap_or(ReasoningEffort::Low),
        }),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffort>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub message: Message,
}

impl Message {
    pub fn system(content: &str) -> Self {
        Message {
            role: "system".to_string(),
            content: content.to_string(),
        }
    }

    pub fn user(content: &str) -> Self {
        Message {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }
}

pub fn build_chat_request(provider: &dyn LlmProvider, messages: Vec<Message>) -> ChatRequest {
    let (max_tokens, max_completion_tokens) = match provider.kind() {
        ProviderKind::OpenAi => (Some(STORY_TOKEN_LIMIT), None),
        ProviderKind::Xai => (None, Some(STORY_TOKEN_LIMIT)),
    };

    ChatRequest {
        model: provider.model().to_string(),
        messages,
        max_tokens,
        max_completion_tokens,
        temperature: Some(STORY_TEMPERATURE),
        response_format: Some(ResponseFormat {
            format_type: "json_object".to_string(),
        }),
        reasoning_effort: provider.reasoning_effort(),
    }
}

pub struct ChatClient {
    http: reqwest::Client,
    provider: Box<dyn LlmProvider>,
}

impl ChatClient {
    pub fn new(provider: Box<dyn LlmProvider>) -> Self {
        Self {
            http: reqwest::Client::new(),
            provider,
        }
    }

    pub fn build_request(&self, messages: Vec<Message>) -> ChatRequest {
        build_chat_request(self.provider.as_ref(), messages)
    }

    pub async fn complete(
        &self,
        request: &ChatRequest,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let response = self
            .http
            .post(self.provider.chat_completions_url())
            .bearer_auth(self.provider.api_key())
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("API Error: {error_text}").into());
        }

        let api_response: ApiResponse = response.json().await?;
        match api_response.choices.first() {
            Some(choice) => Ok(choice.message.content.clone()),
            None => Err("No response from AI".into()),
        }
    }

    pub async fn check_api_key(&self) -> Result<(), Box<dyn std::error::Error>> {
        let response = self
            .http
            .get(self.provider.models_url())
            .bearer_auth(self.provider.api_key())
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::OK => Ok(()),
            status => Err(format!("Invalid API key: {status}").into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection_provider(selection: ProviderSelection) -> Box<dyn LlmProvider> {
        provider_from_selection(selection, "test-key".to_string())
    }

    fn request_json(provider: &dyn LlmProvider) -> serde_json::Value {
        let request = build_chat_request(
            provider,
            vec![
                Message::system("Be a storyteller."),
                Message::user("Look around."),
            ],
        );
        serde_json::to_value(request).expect("request serializes")
    }

    #[test]
    fn missing_provider_stays_on_openai() {
        let selection = select_provider(None, None, None).unwrap();
        assert_eq!(selection.kind, ProviderKind::OpenAi);
        assert_eq!(selection.model, OPENAI_DEFAULT_MODEL);
        assert_eq!(selection.reasoning_effort, None);
    }

    #[test]
    fn blank_provider_stays_on_openai() {
        let selection = select_provider(Some("   "), None, Some("medium")).unwrap();
        assert_eq!(selection.kind, ProviderKind::OpenAi);
        assert_eq!(selection.reasoning_effort, None);
    }

    #[test]
    fn provider_name_is_case_insensitive() {
        let selection = select_provider(Some(" XAI "), None, None).unwrap();
        assert_eq!(selection.kind, ProviderKind::Xai);
        assert_eq!(selection.model, XAI_DEFAULT_MODEL);
        assert_eq!(selection.reasoning_effort, Some(ReasoningEffort::Low));
    }

    #[test]
    fn unknown_provider_is_rejected() {
        let error = select_provider(Some("anthropic"), None, None).unwrap_err();
        assert!(error.contains("anthropic"));
        assert!(error.contains("openai"));
        assert!(error.contains("xai"));
    }

    #[test]
    fn xai_model_and_reasoning_can_be_overridden() {
        let selection = select_provider(Some("xai"), Some(" grok-4 "), Some("Medium")).unwrap();
        assert_eq!(selection.model, "grok-4");
        assert_eq!(selection.reasoning_effort, Some(ReasoningEffort::Medium));
    }

    #[test]
    fn xai_rejects_reasoning_efforts_outside_low_and_medium() {
        let error = select_provider(Some("xai"), None, Some("high")).unwrap_err();
        assert!(error.contains("high"));
        assert!(error.contains("low"));
        assert!(error.contains("medium"));
    }

    #[test]
    fn openai_ignores_reasoning_effort() {
        let selection = select_provider(Some("openai"), None, Some("high")).unwrap();
        assert_eq!(selection.kind, ProviderKind::OpenAi);
        assert_eq!(selection.reasoning_effort, None);
    }

    #[test]
    fn openai_keeps_known_model_ids() {
        for model in [
            "gpt-3.5-turbo",
            "gpt-4o",
            "gpt-4o-mini",
            "gpt-4-turbo",
            "gpt-5",
            "openai-o4-mini",
        ] {
            let selection = select_provider(Some("openai"), Some(model), None).unwrap();
            assert_eq!(selection.model, model);
        }
    }

    #[test]
    fn openai_rejects_unknown_models() {
        let error = select_provider(Some("openai"), Some("grok-4.7"), None).unwrap_err();
        assert!(error.contains("Unknown model"));
        assert!(error.contains("grok-4.7"));
    }

    #[test]
    fn api_key_follows_the_selected_provider() {
        let openai =
            resolve_api_key(ProviderKind::OpenAi, Some(" openai-key "), Some("xai-key")).unwrap();
        let xai =
            resolve_api_key(ProviderKind::Xai, Some("openai-key"), Some(" xai-key ")).unwrap();
        assert_eq!(openai, "openai-key");
        assert_eq!(xai, "xai-key");
    }

    #[test]
    fn missing_xai_key_names_the_env_var_and_hides_other_secrets() {
        let error = resolve_api_key(ProviderKind::Xai, Some("super-secret-openai"), Some("   "))
            .unwrap_err();
        assert!(error.contains("XAI_API_KEY"));
        assert!(!error.contains("super-secret-openai"));
    }

    #[test]
    fn missing_openai_key_names_its_env_var() {
        let error = resolve_api_key(ProviderKind::OpenAi, None, Some("xai-key")).unwrap_err();
        assert!(error.contains("OPENAI_API_KEY"));
        assert!(!error.contains("xai-key"));
    }

    #[test]
    fn providers_point_at_their_chat_completions_endpoints() {
        assert_eq!(
            ProviderKind::OpenAi.chat_completions_url(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            ProviderKind::Xai.chat_completions_url(),
            "https://api.x.ai/v1/chat/completions"
        );
        assert_eq!(
            ProviderKind::OpenAi.models_url(),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(ProviderKind::Xai.models_url(), "https://api.x.ai/v1/models");

        let openai = selection_provider(select_provider(Some("openai"), None, None).unwrap());
        let xai = selection_provider(select_provider(Some("xai"), None, Some("low")).unwrap());
        assert_eq!(openai.chat_completions_url(), OPENAI_CHAT_COMPLETIONS_URL);
        assert_eq!(xai.chat_completions_url(), XAI_CHAT_COMPLETIONS_URL);
        assert_eq!(openai.models_url(), OPENAI_MODELS_URL);
        assert_eq!(xai.models_url(), XAI_MODELS_URL);
    }

    #[test]
    fn openai_request_matches_the_original_chat_body() {
        let provider = selection_provider(select_provider(Some("openai"), None, None).unwrap());
        let body = request_json(provider.as_ref());

        let mut keys: Vec<&str> = body
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "max_tokens",
                "messages",
                "model",
                "response_format",
                "temperature"
            ]
        );
        assert_eq!(body["model"], "gpt-4o-mini");
        assert_eq!(body["max_tokens"], 3000);
        assert_eq!(body["response_format"]["type"], "json_object");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "Be a storyteller.");
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(body["messages"][1]["content"], "Look around.");
        let temperature = body["temperature"].as_f64().unwrap();
        assert!((temperature - 0.9).abs() < 1e-6);
        assert!(!body.to_string().contains("test-key"));
    }

    #[test]
    fn xai_request_sets_grok_model_and_reasoning_effort() {
        let provider =
            selection_provider(select_provider(Some("xai"), None, Some("medium")).unwrap());
        assert_eq!(provider.kind(), ProviderKind::Xai);
        assert_eq!(provider.api_key(), "test-key");
        assert_eq!(provider.reasoning_effort(), Some(ReasoningEffort::Medium));

        let body = request_json(provider.as_ref());
        let mut keys: Vec<&str> = body
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "max_completion_tokens",
                "messages",
                "model",
                "reasoning_effort",
                "response_format",
                "temperature"
            ]
        );
        assert_eq!(body["model"], "grok-4.7");
        assert_eq!(body["reasoning_effort"], "medium");
        assert_eq!(body["max_completion_tokens"], 3000);
        assert!(body.get("max_tokens").is_none());
        assert_eq!(body["response_format"]["type"], "json_object");
        assert_eq!(body["messages"][1]["content"], "Look around.");
        assert!(!body.to_string().contains("test-key"));
    }

    #[test]
    fn default_xai_request_uses_low_reasoning_effort() {
        let provider = selection_provider(select_provider(Some("xai"), None, None).unwrap());
        let body = request_json(provider.as_ref());
        assert_eq!(body["model"], "grok-4.7");
        assert_eq!(body["reasoning_effort"], "low");
    }
}
