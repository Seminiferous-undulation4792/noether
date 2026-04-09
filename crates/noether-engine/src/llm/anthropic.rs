//! Anthropic API provider.
//!
//! Calls `api.anthropic.com/v1/messages` directly.
//! Auth: `ANTHROPIC_API_KEY` environment variable.
//!
//! Note: Anthropic does not offer an embeddings API, so this module
//! only provides an LLM provider.

use crate::llm::{LlmConfig, LlmError, LlmProvider, Message, Role};
use serde_json::{json, Value};

const ANTHROPIC_API_BASE: &str = "https://api.anthropic.com/v1";
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
const ANTHROPIC_VERSION: &str = "2023-06-01";

// ── LLM provider ────────────────────────────────────────────────────────────

/// Calls `api.anthropic.com/v1/messages` with an API key.
///
/// Supports all Anthropic Claude models:
/// - `claude-sonnet-4-20250514` — balanced (default)
/// - `claude-opus-4-20250514` — most capable
/// - `claude-haiku-3-20250414` — fastest, cheapest
///
/// Set `ANTHROPIC_API_KEY` to your API key from console.anthropic.com.
/// Override model with `ANTHROPIC_MODEL`.
pub struct AnthropicProvider {
    api_key: String,
    client: reqwest::blocking::Client,
}

impl AnthropicProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("failed to build reqwest client");
        Self {
            api_key: api_key.into(),
            client,
        }
    }

    /// Construct from environment. Returns `Err` if `ANTHROPIC_API_KEY` is not set.
    pub fn from_env() -> Result<Self, String> {
        let key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "ANTHROPIC_API_KEY is not set".to_string())?;
        Ok(Self::new(key))
    }
}

impl LlmProvider for AnthropicProvider {
    fn complete(&self, messages: &[Message], config: &LlmConfig) -> Result<String, LlmError> {
        let url = format!("{ANTHROPIC_API_BASE}/messages");

        let model = std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        // Anthropic requires system messages in a top-level `system` field,
        // not in the messages array. Only "user" and "assistant" roles are allowed.
        let mut system_text: Option<String> = None;
        let mut msgs: Vec<Value> = Vec::new();

        for m in messages {
            match m.role {
                Role::System => {
                    // Concatenate multiple system messages if present.
                    match &mut system_text {
                        Some(existing) => {
                            existing.push('\n');
                            existing.push_str(&m.content);
                        }
                        None => {
                            system_text = Some(m.content.clone());
                        }
                    }
                }
                Role::User => {
                    msgs.push(json!({"role": "user", "content": m.content}));
                }
                Role::Assistant => {
                    msgs.push(json!({"role": "assistant", "content": m.content}));
                }
            }
        }

        let mut body = json!({
            "model": model,
            "max_tokens": config.max_tokens,
            "messages": msgs,
        });

        if let Some(sys) = system_text {
            body["system"] = Value::String(sys);
        }

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| LlmError::Http(e.to_string()))?;

        let status = resp.status();
        let text = resp.text().map_err(|e| LlmError::Http(e.to_string()))?;

        if !status.is_success() {
            return Err(LlmError::Provider(format!(
                "Anthropic API HTTP {status}: {text}"
            )));
        }

        let json: Value =
            serde_json::from_str(&text).map_err(|e| LlmError::Parse(e.to_string()))?;

        json["content"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| LlmError::Parse(format!("unexpected Anthropic response shape: {json}")))
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_errors_without_key() {
        let saved = std::env::var("ANTHROPIC_API_KEY").ok();
        std::env::remove_var("ANTHROPIC_API_KEY");
        assert!(AnthropicProvider::from_env().is_err());
        if let Some(k) = saved {
            std::env::set_var("ANTHROPIC_API_KEY", k);
        }
    }

    #[test]
    fn system_message_extraction() {
        // Verify that system messages are separated from user/assistant messages.
        let messages = vec![
            Message::system("You are helpful."),
            Message::user("Hello"),
            Message::assistant("Hi there"),
        ];

        let mut system_text: Option<String> = None;
        let mut msgs: Vec<Value> = Vec::new();

        for m in &messages {
            match m.role {
                Role::System => match &mut system_text {
                    Some(existing) => {
                        existing.push('\n');
                        existing.push_str(&m.content);
                    }
                    None => {
                        system_text = Some(m.content.clone());
                    }
                },
                Role::User => {
                    msgs.push(json!({"role": "user", "content": m.content}));
                }
                Role::Assistant => {
                    msgs.push(json!({"role": "assistant", "content": m.content}));
                }
            }
        }

        assert_eq!(system_text, Some("You are helpful.".to_string()));
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0]["role"], "user");
        assert_eq!(msgs[1]["role"], "assistant");
    }

    #[test]
    fn multiple_system_messages_concatenated() {
        let messages = vec![
            Message::system("First instruction."),
            Message::system("Second instruction."),
            Message::user("Hello"),
        ];

        let mut system_text: Option<String> = None;
        for m in &messages {
            if matches!(m.role, Role::System) {
                match &mut system_text {
                    Some(existing) => {
                        existing.push('\n');
                        existing.push_str(&m.content);
                    }
                    None => {
                        system_text = Some(m.content.clone());
                    }
                }
            }
        }

        assert_eq!(
            system_text,
            Some("First instruction.\nSecond instruction.".to_string())
        );
    }
}
