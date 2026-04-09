//! OpenAI API provider (also works with any OpenAI-compatible API).
//!
//! Auth: `OPENAI_API_KEY` environment variable.
//!
//! Compatible with OpenAI, Ollama, Together AI, and any other service
//! that implements the OpenAI chat/completions and embeddings endpoints.
//!
//! Override the base URL with `OPENAI_API_BASE` for self-hosted or
//! third-party OpenAI-compatible services.

use crate::index::embedding::{Embedding, EmbeddingError, EmbeddingProvider};
use crate::llm::{LlmConfig, LlmError, LlmProvider, Message, Role};
use serde_json::{json, Value};

const DEFAULT_API_BASE: &str = "https://api.openai.com/v1";
const DEFAULT_EMBEDDING_MODEL: &str = "text-embedding-3-small";
const DEFAULT_EMBEDDING_DIMENSIONS: usize = 1536;

// ── LLM provider ────────────────────────────────────────────────────────────

/// Calls `{base}/chat/completions` with an OpenAI-compatible API.
///
/// Supports all OpenAI chat models and any compatible endpoint:
/// - `gpt-4o-mini` — fast and cheap (default)
/// - `gpt-4o` — most capable
/// - Any model exposed by an OpenAI-compatible API
///
/// Set `OPENAI_API_KEY` to your API key.
/// Override model with `OPENAI_MODEL`.
/// Override base URL with `OPENAI_API_BASE` (e.g. `http://localhost:11434/v1` for Ollama).
pub struct OpenAiProvider {
    api_key: String,
    api_base: String,
    client: reqwest::blocking::Client,
}

impl OpenAiProvider {
    pub fn new(api_key: impl Into<String>, api_base: impl Into<String>) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("failed to build reqwest client");
        Self {
            api_key: api_key.into(),
            api_base: api_base.into(),
            client,
        }
    }

    /// Construct from environment. Returns `Err` if `OPENAI_API_KEY` is not set.
    pub fn from_env() -> Result<Self, String> {
        let key =
            std::env::var("OPENAI_API_KEY").map_err(|_| "OPENAI_API_KEY is not set".to_string())?;
        let base =
            std::env::var("OPENAI_API_BASE").unwrap_or_else(|_| DEFAULT_API_BASE.to_string());
        Ok(Self::new(key, base))
    }
}

impl LlmProvider for OpenAiProvider {
    fn complete(&self, messages: &[Message], config: &LlmConfig) -> Result<String, LlmError> {
        let url = format!("{}/chat/completions", self.api_base);

        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| config.model.clone());

        let msgs: Vec<Value> = messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                };
                json!({"role": role, "content": m.content})
            })
            .collect();

        let body = json!({
            "model": model,
            "messages": msgs,
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
            "stream": false,
        });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .map_err(|e| LlmError::Http(e.to_string()))?;

        let status = resp.status();
        let text = resp.text().map_err(|e| LlmError::Http(e.to_string()))?;

        if !status.is_success() {
            return Err(LlmError::Provider(format!(
                "OpenAI API HTTP {status}: {text}"
            )));
        }

        let json: Value =
            serde_json::from_str(&text).map_err(|e| LlmError::Parse(e.to_string()))?;

        json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| LlmError::Parse(format!("unexpected OpenAI response shape: {json}")))
    }
}

// ── Embedding provider ───────────────────────────────────────────────────────

/// Calls `{base}/embeddings` using the OpenAI embeddings API.
///
/// - Default model: `text-embedding-3-small` (1536 dimensions)
/// - Compatible with any OpenAI-compatible embeddings endpoint
pub struct OpenAiEmbeddingProvider {
    api_key: String,
    api_base: String,
    model: String,
    client: reqwest::blocking::Client,
}

impl OpenAiEmbeddingProvider {
    pub fn new(api_key: impl Into<String>, api_base: impl Into<String>) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("failed to build reqwest client");
        Self {
            api_key: api_key.into(),
            api_base: api_base.into(),
            model: std::env::var("OPENAI_EMBEDDING_MODEL")
                .unwrap_or_else(|_| DEFAULT_EMBEDDING_MODEL.into()),
            client,
        }
    }

    /// Construct from environment. Returns `Err` if `OPENAI_API_KEY` is not set.
    pub fn from_env() -> Result<Self, String> {
        let key =
            std::env::var("OPENAI_API_KEY").map_err(|_| "OPENAI_API_KEY is not set".to_string())?;
        let base =
            std::env::var("OPENAI_API_BASE").unwrap_or_else(|_| DEFAULT_API_BASE.to_string());
        Ok(Self::new(key, base))
    }
}

impl EmbeddingProvider for OpenAiEmbeddingProvider {
    fn dimensions(&self) -> usize {
        DEFAULT_EMBEDDING_DIMENSIONS
    }

    fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        let mut batch = self.embed_batch(&[text])?;
        batch
            .pop()
            .ok_or_else(|| EmbeddingError::Provider("empty response".into()))
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!("{}/embeddings", self.api_base);
        let body = json!({
            "model": self.model,
            "input": texts,
            "encoding_format": "float",
        });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .map_err(|e| EmbeddingError::Provider(e.to_string()))?;

        let status = resp.status();
        let text = resp
            .text()
            .map_err(|e| EmbeddingError::Provider(e.to_string()))?;

        if !status.is_success() {
            return Err(EmbeddingError::Provider(format!(
                "OpenAI embeddings HTTP {status}: {text}"
            )));
        }

        let json: Value =
            serde_json::from_str(&text).map_err(|e| EmbeddingError::Provider(e.to_string()))?;

        let mut items: Vec<(usize, Embedding)> = json["data"]
            .as_array()
            .ok_or_else(|| EmbeddingError::Provider("missing 'data' field".into()))?
            .iter()
            .map(|item| {
                let index = item["index"].as_u64().unwrap_or(0) as usize;
                let vec: Embedding = item["embedding"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect();
                (index, vec)
            })
            .collect();

        items.sort_by_key(|(idx, _)| *idx);
        Ok(items.into_iter().map(|(_, v)| v).collect())
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_errors_without_key() {
        let saved = std::env::var("OPENAI_API_KEY").ok();
        std::env::remove_var("OPENAI_API_KEY");
        assert!(OpenAiProvider::from_env().is_err());
        assert!(OpenAiEmbeddingProvider::from_env().is_err());
        if let Some(k) = saved {
            std::env::set_var("OPENAI_API_KEY", k);
        }
    }

    #[test]
    fn default_base_url() {
        let provider = OpenAiProvider::new("test-key", DEFAULT_API_BASE);
        assert_eq!(provider.api_base, "https://api.openai.com/v1");
    }

    #[test]
    fn custom_base_url() {
        let provider = OpenAiProvider::new("test-key", "http://localhost:11434/v1");
        assert_eq!(provider.api_base, "http://localhost:11434/v1");
    }
}
