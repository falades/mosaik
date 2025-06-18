use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::components::ChatMessage;

mod ollama;
mod anthropic;
mod api_manager;

pub use {
    ollama::OllamaClient,
    anthropic::AnthropicClient,
    api_manager::ApiManager
}; 

// Request structure for Ollama's generate API
#[derive(Serialize)]
pub struct LLMRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    think: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<ThinkingConfig>,
}

#[derive(Serialize)]
pub struct ThinkingConfig {
    #[serde(rename = "type")]
    pub thinking_type: String,
    pub budget_tokens: u32
}

#[derive(Deserialize, Debug)]
pub struct LLMResponse {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<ContentDelta>,
}

#[derive(Deserialize, Debug)]
pub struct ContentDelta {
    #[serde(rename = "type")]
    pub delta_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
}

pub trait LLMProvider {
    fn new() -> Self;
    async fn generate(&self, model: Option<String>, messages: Vec<ChatMessage>, thinking: Option<bool>) -> Result<tokio::sync::mpsc::Receiver<ChatMessage>>;
    async fn get_available_models(&self) -> Result<Vec<String>>;
}