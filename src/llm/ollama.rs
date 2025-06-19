use dioxus::logger::tracing::{info, error};
use serde::Deserialize;
use anyhow::{Result, Context};
use crate::components::ChatMessage;
use super::{LLMRequest, LLMProvider};

// Client for interacting with the Ollama API
pub struct OllamaClient {
    client: reqwest::Client,
    base_url: String,
    default_model: String
}

// Response structure from Ollama's generate API
#[derive(Deserialize, Debug)]
pub struct OllamaResponse {
    pub message: ChatMessage,
    pub done: bool
}

impl LLMProvider for OllamaClient {
    /// Create a new Ollama client
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "http://localhost:11434".to_string(),
            default_model: "".to_string()
        }
    }

    /// Generate text using the Ollama API
    async fn generate(&self, model: Option<String>, messages: Vec<ChatMessage>, think: Option<bool>) -> Result<tokio::sync::mpsc::Receiver<ChatMessage>> {
        let model = model.unwrap_or_else(|| self.default_model.clone());
        let url = format!("{}/api/chat", self.base_url);
        
        let request = LLMRequest {
            model,
            messages,
            stream: true,
            max_tokens: None,
            think,
            thinking: None
        };

        info!("Sending request to Ollama API at {}", url);
        
        let mut response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Ollama API")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unable to read error response".to_string());
            error!("Ollama API returned error: Status {}, Content: {}", status, error_text);
            anyhow::bail!("Ollama API error: Status {}: {}", status, error_text);
        }

        let (tx, rx) = tokio::sync::mpsc::channel::<ChatMessage>(100);
        tokio::spawn(async move {
            let mut buffer = String::new();
            
            while let Ok(Some(bytes)) = response.chunk().await {
                buffer.push_str(&String::from_utf8_lossy(&bytes));
                
                while let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim().to_string();
                    buffer.drain(..=line_end);
                    
                    if line.is_empty() {
                        continue;
                    }
                    
                    let Ok(response_chunk) = serde_json::from_str::<OllamaResponse>(&line) else {
                        continue;
                    };
                    
                    if !response_chunk.message.content.is_empty() || response_chunk.message.thinking.is_some() {
                        let _ = tx.send(response_chunk.message).await;
                    }
                    
                    if response_chunk.done {
                        return;
                    }
                }
            }
        }); 
        Ok(rx)
    }
    
    async fn get_available_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.base_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch Ollama models")?;
            
        // Parse Ollama's response format and extract model names
        let models: serde_json::Value = response.json().await?;
        let model_names = models["models"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|m| m["name"].as_str())
            .map(|s| s.to_string())
            .collect();
            
        Ok(model_names)
    }
}