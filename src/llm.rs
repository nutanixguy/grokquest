//! LLM backends for the storyteller.
//!
//! Game code depends on [`LlmProvider`]. [`OpenAiProvider`] is the original
//! OpenAI chat-completions client and also talks to any OpenAI-compatible
//! endpoint. [`XaiProvider`] is the xAI Grok client.
//!
//! `AI_PROVIDER` selects `openai` (the default), `xai`, or `compatible`.
//! `LLM_PROVIDER` is still accepted. `AI_BASE_URL` and `AI_MODEL` override the
//! API root and model, so another OpenAI-compatible server needs no code change.

use serde::{Deserialize, Serialize};

use crate::gpt_model::GptModel;

pub const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
pub const XAI_BASE_URL: &str = "https://api.x.ai/v1";
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
    Compatible,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Xai => "xai",
            Self::Compatible => "compatible",
        }
    }

    pub fn default_base_url(self) -> Option<&'static str> {
        match self {
            Self::OpenAi => Some(OPENAI_BASE_URL),
            Self::Xai => Some(XAI_BASE_URL),
            Self::Compatible => None,
        }
    }

    pub fn default_model(self) -> Option<&'static str> {
        match self {
            Self::OpenAi => Some(OPENAI_DEFAULT_MODEL),
            Self::Xai => Some(XAI_DEFAULT_MODEL),
            Self::Compatible => None,
        }
    }

    pub fn api_key_env(self) -> &'static str {
        match self {
            Self::OpenAi => "OPENAI_API_KEY",
            Self::Xai => "XAI_API_KEY",
            Self::Compatible => "AI_API_KEY",
        }
    }
}

impl std::str::FromStr for ProviderKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "openai" => Ok(Self::OpenAi),
            "xai" => Ok(Self::Xai),
            "compatible" => Ok(Self::Compatible),
            other => Err(format!(
                "unknown LLM provider '{other}'. Expected 'openai', 'xai', or 'compatible'."
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
    pub base_url: String,
}

/// First non-empty value wins: explicit flag, then preferred env, then fallback env.
pub fn choose_setting<'a>(
    cli: Option<&'a str>,
    preferred_env: Option<&'a str>,
    fallback_env: Option<&'a str>,
) -> Option<&'a str> {
    [cli, preferred_env, fallback_env]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| !value.is_empty())
}

/// Choose a backend from CLI or environment values.
///
/// A missing or blank provider stays on OpenAI. `compatible` is any
/// OpenAI-compatible API root and requires a base URL and a model id.
/// xAI defaults to `grok-4.7` and `low` reasoning effort.
pub fn select_provider(
    provider: Option<&str>,
    model: Option<&str>,
    reasoning_effort: Option<&str>,
    base_url: Option<&str>,
) -> Result<ProviderSelection, String> {
    let kind = match provider.map(str::trim).filter(|value| !value.is_empty()) {
        None => ProviderKind::OpenAi,
        Some(value) => value.parse()?,
    };

    let builtin_base = kind.default_base_url();
    let base_url = match base_url.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => normalize_base_url(value)?,
        None => match builtin_base {
            Some(default) => default.to_string(),
            None => {
                return Err(
                    "the compatible provider needs a base URL. Set AI_BASE_URL or pass --base-url."
                        .to_string(),
                );
            }
        },
    };
    let using_builtin_base = builtin_base == Some(base_url.as_str());

    let model = match model.map(str::trim).filter(|value| !value.is_empty()) {
        Some(explicit) => canonicalize_model(kind, explicit, using_builtin_base)?,
        None => match kind.default_model() {
            Some(default) => default.to_string(),
            None => {
                return Err(
                    "the compatible provider needs a model id. Set AI_MODEL or LLM_MODEL, or pass --model."
                        .to_string(),
                );
            }
        },
    };

    let reasoning_effort = match kind {
        ProviderKind::OpenAi | ProviderKind::Compatible => None,
        ProviderKind::Xai => Some(parse_optional_reasoning(reasoning_effort)?),
    };

    Ok(ProviderSelection {
        kind,
        model,
        reasoning_effort,
        base_url,
    })
}

fn parse_optional_reasoning(value: Option<&str>) -> Result<ReasoningEffort, String> {
    match value.map(str::trim).filter(|effort| !effort.is_empty()) {
        None => Ok(ReasoningEffort::Low),
        Some(effort) => effort.parse(),
    }
}

fn canonicalize_model(
    kind: ProviderKind,
    model: &str,
    using_builtin_base: bool,
) -> Result<String, String> {
    match kind {
        ProviderKind::OpenAi => {
            if using_builtin_base {
                let parsed = GptModel::try_from(model)?;
                Ok(parsed.as_str().to_string())
            } else {
                Ok(model.to_string())
            }
        }
        ProviderKind::Xai => Ok(model.to_string()),
        ProviderKind::Compatible => Ok(model.to_string()),
    }
}

fn normalize_base_url(value: &str) -> Result<String, String> {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.is_empty() || !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
        return Err(
            "base URL must start with http:// or https://. Set AI_BASE_URL to an API root such as https://api.example.com/v1."
                .to_string(),
        );
    }
    Ok(trimmed.to_string())
}

fn endpoint_url(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

pub fn resolve_api_key(
    kind: ProviderKind,
    openai_api_key: Option<&str>,
    xai_api_key: Option<&str>,
    ai_api_key: Option<&str>,
) -> Result<String, String> {
    let value = match kind {
        ProviderKind::OpenAi => openai_api_key,
        ProviderKind::Xai => xai_api_key,
        ProviderKind::Compatible => ai_api_key,
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
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn api_key(&self) -> &str;
    fn chat_completions_url(&self) -> &str;
    fn models_url(&self) -> &str;
    fn reasoning_effort(&self) -> Option<ReasoningEffort>;

    /// xAI bills reasoning separately, so the story budget uses `max_completion_tokens`.
    fn uses_max_completion_tokens(&self) -> bool {
        false
    }
}

/// OpenAI chat completions. Also used for any compatible endpoint.
pub struct OpenAiProvider {
    name: &'static str,
    api_key: String,
    model: String,
    chat_completions_url: String,
    models_url: String,
}

pub struct XaiProvider {
    api_key: String,
    model: String,
    chat_completions_url: String,
    models_url: String,
    reasoning_effort: ReasoningEffort,
}

impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str {
        self.name
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn api_key(&self) -> &str {
        &self.api_key
    }

    fn chat_completions_url(&self) -> &str {
        &self.chat_completions_url
    }

    fn models_url(&self) -> &str {
        &self.models_url
    }

    fn reasoning_effort(&self) -> Option<ReasoningEffort> {
        None
    }
}

impl LlmProvider for XaiProvider {
    fn name(&self) -> &str {
        ProviderKind::Xai.as_str()
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn api_key(&self) -> &str {
        &self.api_key
    }

    fn chat_completions_url(&self) -> &str {
        &self.chat_completions_url
    }

    fn models_url(&self) -> &str {
        &self.models_url
    }

    fn reasoning_effort(&self) -> Option<ReasoningEffort> {
        Some(self.reasoning_effort)
    }

    fn uses_max_completion_tokens(&self) -> bool {
        true
    }
}

/// Build the selected backend.
///
/// A new provider is three steps: implement [`LlmProvider`], match its name
/// here and in [`ProviderKind`], and read its key in [`resolve_api_key`].
pub fn provider_from_selection(
    selection: ProviderSelection,
    api_key: String,
) -> Box<dyn LlmProvider> {
    let chat_completions_url = endpoint_url(&selection.base_url, "chat/completions");
    let models_url = endpoint_url(&selection.base_url, "models");
    match selection.kind {
        ProviderKind::OpenAi => Box::new(OpenAiProvider {
            name: ProviderKind::OpenAi.as_str(),
            api_key,
            model: selection.model,
            chat_completions_url,
            models_url,
        }),
        ProviderKind::Compatible => Box::new(OpenAiProvider {
            name: ProviderKind::Compatible.as_str(),
            api_key,
            model: selection.model,
            chat_completions_url,
            models_url,
        }),
        ProviderKind::Xai => Box::new(XaiProvider {
            api_key,
            model: selection.model,
            chat_completions_url,
            models_url,
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
    let (max_tokens, max_completion_tokens) = if provider.uses_max_completion_tokens() {
        (None, Some(STORY_TOKEN_LIMIT))
    } else {
        (Some(STORY_TOKEN_LIMIT), None)
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

    fn select(
        provider: Option<&str>,
        model: Option<&str>,
        reasoning_effort: Option<&str>,
    ) -> Result<ProviderSelection, String> {
        select_provider(provider, model, reasoning_effort, None)
    }

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
        let selection = select(None, None, None).unwrap();
        assert_eq!(selection.kind, ProviderKind::OpenAi);
        assert_eq!(selection.model, OPENAI_DEFAULT_MODEL);
        assert_eq!(selection.reasoning_effort, None);
        assert_eq!(selection.base_url, OPENAI_BASE_URL);
    }

    #[test]
    fn blank_provider_stays_on_openai() {
        let selection = select(Some("   "), None, Some("medium")).unwrap();
        assert_eq!(selection.kind, ProviderKind::OpenAi);
        assert_eq!(selection.reasoning_effort, None);
    }

    #[test]
    fn provider_name_is_case_insensitive() {
        let selection = select(Some(" XAI "), None, None).unwrap();
        assert_eq!(selection.kind, ProviderKind::Xai);
        assert_eq!(selection.model, XAI_DEFAULT_MODEL);
        assert_eq!(selection.reasoning_effort, Some(ReasoningEffort::Low));
        assert_eq!(selection.base_url, XAI_BASE_URL);
    }

    #[test]
    fn unknown_provider_is_rejected() {
        let error = select(Some("anthropic"), None, None).unwrap_err();
        assert!(error.contains("anthropic"));
        assert!(error.contains("openai"));
        assert!(error.contains("xai"));
        assert!(error.contains("compatible"));
    }

    #[test]
    fn xai_model_and_reasoning_can_be_overridden() {
        let selection = select(Some("xai"), Some(" grok-4 "), Some("Medium")).unwrap();
        assert_eq!(selection.model, "grok-4");
        assert_eq!(selection.reasoning_effort, Some(ReasoningEffort::Medium));
    }

    #[test]
    fn xai_rejects_reasoning_efforts_outside_low_and_medium() {
        let error = select(Some("xai"), None, Some("high")).unwrap_err();
        assert!(error.contains("high"));
        assert!(error.contains("low"));
        assert!(error.contains("medium"));
    }

    #[test]
    fn openai_ignores_reasoning_effort() {
        let selection = select(Some("openai"), None, Some("high")).unwrap();
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
            let selection = select(Some("openai"), Some(model), None).unwrap();
            assert_eq!(selection.model, model);
        }
    }

    #[test]
    fn openai_rejects_unknown_models() {
        let error = select(Some("openai"), Some("grok-4.7"), None).unwrap_err();
        assert!(error.contains("Unknown model"));
        assert!(error.contains("grok-4.7"));
    }

    #[test]
    fn api_key_follows_the_selected_provider() {
        let openai = resolve_api_key(
            ProviderKind::OpenAi,
            Some(" openai-key "),
            Some("xai-key"),
            Some("ai-key"),
        )
        .unwrap();
        let xai = resolve_api_key(
            ProviderKind::Xai,
            Some("openai-key"),
            Some(" xai-key "),
            Some("ai-key"),
        )
        .unwrap();
        let compatible = resolve_api_key(
            ProviderKind::Compatible,
            Some("openai-key"),
            Some("xai-key"),
            Some(" ai-key "),
        )
        .unwrap();
        assert_eq!(openai, "openai-key");
        assert_eq!(xai, "xai-key");
        assert_eq!(compatible, "ai-key");
    }

    #[test]
    fn missing_xai_key_names_the_env_var_and_hides_other_secrets() {
        let error = resolve_api_key(
            ProviderKind::Xai,
            Some("super-secret-openai"),
            Some("   "),
            Some("super-secret-compatible"),
        )
        .unwrap_err();
        assert!(error.contains("XAI_API_KEY"));
        assert!(!error.contains("super-secret-openai"));
        assert!(!error.contains("super-secret-compatible"));
    }

    #[test]
    fn missing_openai_key_names_its_env_var() {
        let error = resolve_api_key(ProviderKind::OpenAi, None, Some("xai-key"), Some("ai-key"))
            .unwrap_err();
        assert!(error.contains("OPENAI_API_KEY"));
        assert!(!error.contains("xai-key"));
        assert!(!error.contains("ai-key"));
    }

    #[test]
    fn missing_compatible_key_names_its_env_var() {
        let error = resolve_api_key(
            ProviderKind::Compatible,
            Some("super-secret-openai"),
            Some("super-secret-xai"),
            Some("  "),
        )
        .unwrap_err();
        assert!(error.contains("AI_API_KEY"));
        assert!(!error.contains("super-secret-openai"));
        assert!(!error.contains("super-secret-xai"));
    }

    #[test]
    fn providers_point_at_their_chat_completions_endpoints() {
        assert_eq!(
            endpoint_url(OPENAI_BASE_URL, "chat/completions"),
            OPENAI_CHAT_COMPLETIONS_URL
        );
        assert_eq!(
            endpoint_url(XAI_BASE_URL, "chat/completions"),
            XAI_CHAT_COMPLETIONS_URL
        );
        assert_eq!(endpoint_url(OPENAI_BASE_URL, "models"), OPENAI_MODELS_URL);
        assert_eq!(endpoint_url(XAI_BASE_URL, "models"), XAI_MODELS_URL);

        let openai = selection_provider(select(Some("openai"), None, None).unwrap());
        let xai = selection_provider(select(Some("xai"), None, Some("low")).unwrap());
        assert_eq!(openai.name(), "openai");
        assert_eq!(xai.name(), "xai");
        assert_eq!(openai.chat_completions_url(), OPENAI_CHAT_COMPLETIONS_URL);
        assert_eq!(xai.chat_completions_url(), XAI_CHAT_COMPLETIONS_URL);
        assert_eq!(openai.models_url(), OPENAI_MODELS_URL);
        assert_eq!(xai.models_url(), XAI_MODELS_URL);
    }

    #[test]
    fn openai_request_matches_the_original_chat_body() {
        let provider = selection_provider(select(Some("openai"), None, None).unwrap());
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
        let provider = selection_provider(select(Some("xai"), None, Some("medium")).unwrap());
        assert_eq!(provider.name(), "xai");
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
        let provider = selection_provider(select(Some("xai"), None, None).unwrap());
        let body = request_json(provider.as_ref());
        assert_eq!(body["model"], "grok-4.7");
        assert_eq!(body["reasoning_effort"], "low");
    }

    #[test]
    fn choose_setting_prefers_the_flag_then_ai_env_then_llm_env() {
        assert_eq!(
            choose_setting(Some(" openai "), Some("xai"), Some("compatible")),
            Some("openai")
        );
        assert_eq!(
            choose_setting(Some("  "), Some(" xai "), Some("openai")),
            Some("xai")
        );
        assert_eq!(
            choose_setting(None, None, Some(" compatible ")),
            Some("compatible")
        );
        assert_eq!(choose_setting(Some(""), Some(" "), None), None);
    }

    #[test]
    fn compatible_provider_uses_the_given_base_url_and_model() {
        let selection = select_provider(
            Some(" Compatible "),
            Some(" llama-3.1-8b "),
            Some("high"),
            Some(" https://example.test/v1/ "),
        )
        .unwrap();
        assert_eq!(selection.kind, ProviderKind::Compatible);
        assert_eq!(selection.model, "llama-3.1-8b");
        assert_eq!(selection.reasoning_effort, None);
        assert_eq!(selection.base_url, "https://example.test/v1");

        let provider = selection_provider(selection);
        assert_eq!(provider.name(), "compatible");
        assert_eq!(
            provider.chat_completions_url(),
            "https://example.test/v1/chat/completions"
        );
        assert_eq!(provider.models_url(), "https://example.test/v1/models");
        assert!(!provider.uses_max_completion_tokens());

        let body = request_json(provider.as_ref());
        assert_eq!(body["model"], "llama-3.1-8b");
        assert_eq!(body["max_tokens"], 3000);
        assert!(body.get("reasoning_effort").is_none());
        assert!(body.get("max_completion_tokens").is_none());
        assert!(!body.to_string().contains("test-key"));
    }

    #[test]
    fn compatible_requires_a_base_url_and_a_model() {
        let missing_base =
            select_provider(Some("compatible"), Some("llama"), None, None).unwrap_err();
        assert!(missing_base.contains("AI_BASE_URL"));
        assert!(!missing_base.contains("llama"));

        let missing_model = select_provider(
            Some("compatible"),
            None,
            None,
            Some("https://example.test/v1"),
        )
        .unwrap_err();
        assert!(missing_model.contains("AI_MODEL"));
        assert!(!missing_model.contains("https://example.test"));
    }

    #[test]
    fn base_url_must_be_http() {
        let error = select_provider(
            Some("compatible"),
            Some("llama"),
            None,
            Some("ftp://example.test/v1"),
        )
        .unwrap_err();
        assert!(error.contains("http://"));
        assert!(!error.contains("ftp://example.test"));
    }

    #[test]
    fn base_url_override_keeps_the_named_provider() {
        let openai = select_provider(
            Some("openai"),
            Some("local-model"),
            Some("medium"),
            Some("http://127.0.0.1:8080/v1/"),
        )
        .unwrap();
        assert_eq!(openai.kind, ProviderKind::OpenAi);
        assert_eq!(openai.model, "local-model");
        assert_eq!(openai.reasoning_effort, None);
        let openai_provider = selection_provider(openai);
        assert_eq!(openai_provider.name(), "openai");
        assert_eq!(
            openai_provider.chat_completions_url(),
            "http://127.0.0.1:8080/v1/chat/completions"
        );
        let openai_body = request_json(openai_provider.as_ref());
        assert_eq!(openai_body["max_tokens"], 3000);
        assert!(openai_body.get("reasoning_effort").is_none());

        let xai = select_provider(
            Some("xai"),
            None,
            Some("medium"),
            Some("https://proxy.example/v1"),
        )
        .unwrap();
        assert_eq!(xai.kind, ProviderKind::Xai);
        assert_eq!(xai.model, XAI_DEFAULT_MODEL);
        assert_eq!(xai.reasoning_effort, Some(ReasoningEffort::Medium));
        let xai_provider = selection_provider(xai);
        assert_eq!(
            xai_provider.chat_completions_url(),
            "https://proxy.example/v1/chat/completions"
        );
        let xai_body = request_json(xai_provider.as_ref());
        assert_eq!(xai_body["model"], "grok-4.7");
        assert_eq!(xai_body["reasoning_effort"], "medium");
        assert_eq!(xai_body["max_completion_tokens"], 3000);
    }
}
