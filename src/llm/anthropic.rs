use dioxus::logger::tracing::{info, error};
use anyhow::{Result, Context};
use crate::{
    llm::ApiManager, 
    components::{ChatMessage, MessageRole}, llm::ThinkingConfig
};
use super::{LLMProvider, LLMRequest, LLMResponse};

pub struct AnthropicClient {
    client: reqwest::Client,
    base_url: String,
    default_model: String,
    api_key: Option<String>,
}

impl LLMProvider for AnthropicClient {
    fn new() -> Self {
        let api_key = if let Ok(key_manager) = ApiManager::new() {
            key_manager.get_anthropic_key().ok()
        } else {
            None
        };
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            default_model: "claude-sonnet-4-20250514".to_string(),
            api_key,
        }
    }

    async fn generate(&self, model: Option<String>, messages: Vec<ChatMessage>, thinking: Option<bool>) -> Result<tokio::sync::mpsc::Receiver<ChatMessage>> {
        let model = model.unwrap_or_else(|| self.default_model.clone());
        let url = format!("{}/messages", self.base_url);
        
        let thinking_config = match thinking {
            Some(true) => {
                Some(ThinkingConfig {
                    thinking_type: "enabled".to_string(),
                    budget_tokens: 2000
                })
            }
            _ => None
        };
        
        let request = LLMRequest {
            model,
            messages,
            stream: true,
            max_tokens: Some(32000),
            think: None,
            thinking: thinking_config
        };

        info!("Sending request to Anthropic API");
        
        let api_key = self.api_key.as_ref()
            .context("Anthropic API key not configured")?;
        let mut response = self.client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Anthropic API")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unable to read error response".to_string());
            error!("Anthropic API returned error: Status {}, Content: {}", status, error_text);
            anyhow::bail!("Anthropic API error: Status {}: {}", status, error_text);
        }

        let (tx, rx) = tokio::sync::mpsc::channel::<ChatMessage>(100);
        tokio::spawn(async move {
            let mut buffer = String::new();
            
            while let Ok(Some(bytes)) = response.chunk().await {
                buffer.push_str(&String::from_utf8_lossy(&bytes));
                
                while let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim().to_string();
                    buffer.drain(..=line_end);
                    
                    if !line.starts_with("data: ") {
                        continue;
                    }
                    
                    let json_part = &line[6..];
                    let Ok(event) = serde_json::from_str::<LLMResponse>(json_part) else {
                        continue;
                    };
                    
                    if event.event_type != "content_block_delta" {
                        continue;
                    }
                    
                    let Some(delta) = event.delta else { continue; };
                    let message = match delta.delta_type.as_str() {
                        "text_delta" => {
                            let Some(text) = delta.text else { continue; };
                            if text.is_empty() { continue; }
                            ChatMessage {
                                role: MessageRole::Assistant,
                                content: text,
                                thinking: None
                            }
                        },
                        "thinking_delta" => {
                            let Some(thinking) = delta.thinking else { continue; };
                            if thinking.is_empty() { continue; }
                            ChatMessage {
                                role: MessageRole::Assistant,
                                content: String::new(),
                                thinking: Some(thinking)
                            }
                        },
                        _ => continue,
                    };
                    let _ = tx.send(message).await;
                }
            }
        }); 
        Ok(rx)
    }
    
    async fn get_available_models(&self) -> Result<Vec<String>> {
        // Anthropic doesn't have a public models endpoint, so return known models
        Ok(vec![
            "claude-sonnet-4-20250514".to_string(),
            "claude-opus-4-20250514".to_string(),
            "claude-3-5-haiku-20241022".to_string(),
        ])
    }
}