use super::{LlmConfig, LlmError, LlmProvider, Message, Role};
use crate::index::embedding::{Embedding, EmbeddingError, EmbeddingProvider};
use serde_json::json;

/// Configuration for Vertex AI.
#[derive(Debug, Clone)]
pub struct VertexAiConfig {
    pub project: String,
    pub location: String,
    pub token: String,
}

impl VertexAiConfig {
    /// Load from environment variables.
    pub fn from_env() -> Result<Self, String> {
        let project = std::env::var("VERTEX_AI_PROJECT").unwrap_or_else(|_| "a2p-common".into());
        let location = std::env::var("VERTEX_AI_LOCATION").unwrap_or_else(|_| "global".into());
        let token = std::env::var("VERTEX_AI_TOKEN").map_err(|_| "VERTEX_AI_TOKEN not set")?;
        Ok(Self {
            project,
            location,
            token,
        })
    }
}

/// Vertex AI LLM provider for Gemini models.
/// Uses the global endpoint: https://aiplatform.googleapis.com/v1/...
pub struct VertexAiLlmProvider {
    config: VertexAiConfig,
    client: reqwest::blocking::Client,
}

impl VertexAiLlmProvider {
    pub fn new(config: VertexAiConfig) -> Self {
        Self {
            config,
            client: reqwest::blocking::Client::new(),
        }
    }

    fn base_url(&self) -> String {
        if self.config.location == "global" {
            "https://aiplatform.googleapis.com/v1".into()
        } else {
            format!(
                "https://{}-aiplatform.googleapis.com/v1",
                self.config.location
            )
        }
    }
}

impl LlmProvider for VertexAiLlmProvider {
    fn complete(&self, messages: &[Message], config: &LlmConfig) -> Result<String, LlmError> {
        let url = format!(
            "{base}/projects/{project}/locations/{location}/publishers/google/models/{model}:generateContent",
            base = self.base_url(),
            project = self.config.project,
            location = self.config.location,
            model = config.model,
        );

        // Convert messages to Gemini format
        let system_instruction: Option<String> = messages
            .iter()
            .find(|m| matches!(m.role, Role::System))
            .map(|m| m.content.clone());

        let contents: Vec<serde_json::Value> = messages
            .iter()
            .filter(|m| !matches!(m.role, Role::System))
            .map(|m| {
                let role = match m.role {
                    Role::User => "user",
                    Role::Assistant => "model",
                    Role::System => unreachable!(),
                };
                json!({
                    "role": role,
                    "parts": [{"text": m.content}]
                })
            })
            .collect();

        let mut body = json!({
            "contents": contents,
            "generationConfig": {
                "maxOutputTokens": config.max_tokens,
                "temperature": config.temperature,
            }
        });

        if let Some(sys) = system_instruction {
            body["systemInstruction"] = json!({
                "parts": [{"text": sys}]
            });
        }

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.config.token)
            .json(&body)
            .send()
            .map_err(|e| LlmError::Http(e.to_string()))?;

        let status = response.status();
        let text = response.text().map_err(|e| LlmError::Http(e.to_string()))?;

        if !status.is_success() {
            return Err(LlmError::Provider(format!("HTTP {status}: {text}")));
        }

        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| LlmError::Parse(e.to_string()))?;

        // Extract text from Gemini response
        json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| LlmError::Parse(format!("unexpected response format: {json}")))
    }
}

/// Vertex AI embedding provider.
/// Uses the global endpoint by default.
pub struct VertexAiEmbeddingProvider {
    config: VertexAiConfig,
    model: String,
    dimensions: usize,
    client: reqwest::blocking::Client,
}

impl VertexAiEmbeddingProvider {
    pub fn new(config: VertexAiConfig, model: Option<String>, dimensions: Option<usize>) -> Self {
        Self {
            config,
            model: model.unwrap_or_else(|| "text-embedding-005".into()),
            dimensions: dimensions.unwrap_or(256),
            client: reqwest::blocking::Client::new(),
        }
    }

    fn base_url(&self) -> String {
        if self.config.location == "global" {
            "https://aiplatform.googleapis.com/v1".into()
        } else {
            format!(
                "https://{}-aiplatform.googleapis.com/v1",
                self.config.location
            )
        }
    }
}

impl EmbeddingProvider for VertexAiEmbeddingProvider {
    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        let url = format!(
            "{base}/projects/{project}/locations/{location}/publishers/google/models/{model}:predict",
            base = self.base_url(),
            project = self.config.project,
            location = self.config.location,
            model = self.model,
        );

        let body = json!({
            "instances": [{"content": text}],
            "parameters": {"outputDimensionality": self.dimensions}
        });

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.config.token)
            .json(&body)
            .send()
            .map_err(|e| EmbeddingError::Provider(e.to_string()))?;

        let status = response.status();
        let text = response
            .text()
            .map_err(|e| EmbeddingError::Provider(e.to_string()))?;

        if !status.is_success() {
            return Err(EmbeddingError::Provider(format!("HTTP {status}: {text}")));
        }

        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| EmbeddingError::Provider(e.to_string()))?;

        let values = json["predictions"][0]["embeddings"]["values"]
            .as_array()
            .ok_or_else(|| EmbeddingError::Provider("unexpected response format".into()))?;

        values
            .iter()
            .map(|v| {
                v.as_f64()
                    .map(|f| f as f32)
                    .ok_or_else(|| EmbeddingError::Provider("non-numeric embedding value".into()))
            })
            .collect()
    }
}
