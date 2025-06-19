use keyring::Entry;
use anyhow::{Result, Context};

pub struct ApiManager {
    openai_entry: Entry,
    anthropic_entry: Entry,
    google_entry: Entry,
}

impl ApiManager {
    pub fn new() -> Result<Self> {
        Ok(ApiManager {
            openai_entry: Entry::new("mosaik", "openai-api-key")
                .context("Failed to create OpenAI keyring entry")?,
            anthropic_entry: Entry::new("mosaik", "anthropic-api-key")
                .context("Failed to create Anthropic keyring entry")?,
            google_entry: Entry::new("mosaik", "google-api-key")
                .context("Failed to create Google keyring entry")?,
        })
    }
    
    pub fn _save_openai_key(&self, key: &str) -> Result<()> {
        self.openai_entry.set_password(key)
            .context("Failed to save OpenAI API key")
    }
    
    pub fn get_openai_key(&self) -> Result<String> {
        self.openai_entry.get_password()
            .context("Failed to retrieve OpenAI API key")
    }
    
    pub fn save_anthropic_key(&self, key: &str) -> Result<()> {
        self.anthropic_entry.set_password(key)
            .context("Failed to save Anthropic API key")
    }
    
    pub fn get_anthropic_key(&self) -> Result<String> {
        self.anthropic_entry.get_password()
            .context("Failed to retrieve Anthropic API key")
    }
    
    pub fn _save_google_key(&self, key: &str) -> Result<()> {
        self.google_entry.set_password(key)
            .context("Failed to save Google API key")
    }
    
    pub fn get_google_key(&self) -> Result<String> {
        self.google_entry.get_password()
            .context("Failed to retrieve Google API key")
    }
}