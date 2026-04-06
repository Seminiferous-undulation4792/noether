pub mod prompt;

use crate::checker::check_graph;
use crate::index::SemanticIndex;
use crate::lagrange::{parse_graph, CompositionGraph};
use crate::llm::{LlmConfig, LlmProvider, Message};
use noether_store::StageStore;
use prompt::{build_system_prompt, extract_json};

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("search failed: {0}")]
    Search(String),
    #[error("LLM call failed: {0}")]
    Llm(#[from] crate::llm::LlmError),
    #[error("no JSON found in LLM response")]
    NoJsonInResponse,
    #[error("invalid graph JSON: {0}")]
    InvalidGraph(String),
    #[error("type check failed after {attempts} attempts: {errors}")]
    TypeCheckFailed { attempts: u32, errors: String },
}

/// Result from the Composition Agent.
pub struct ComposeResult {
    pub graph: CompositionGraph,
    pub attempts: u32,
}

/// The Composition Agent translates problem descriptions into valid composition graphs.
pub struct CompositionAgent<'a> {
    index: &'a SemanticIndex,
    llm: &'a dyn LlmProvider,
    llm_config: LlmConfig,
    max_retries: u32,
}

impl<'a> CompositionAgent<'a> {
    pub fn new(
        index: &'a SemanticIndex,
        llm: &'a dyn LlmProvider,
        llm_config: LlmConfig,
        max_retries: u32,
    ) -> Self {
        Self {
            index,
            llm,
            llm_config,
            max_retries,
        }
    }

    /// Translate a problem description into a valid composition graph.
    pub fn compose(
        &self,
        problem: &str,
        store: &dyn StageStore,
    ) -> Result<ComposeResult, AgentError> {
        // 1. Search for candidate stages
        let search_results = self
            .index
            .search(problem, 20)
            .map_err(|e| AgentError::Search(e.to_string()))?;

        // 2. Resolve stages from store for the prompt
        let candidates: Vec<_> = search_results
            .iter()
            .filter_map(|r| {
                store
                    .get(&r.stage_id)
                    .ok()
                    .flatten()
                    .map(|stage| (r, stage))
            })
            .collect();

        // 3. Build system prompt
        let system_prompt = build_system_prompt(&candidates);

        // 4. Attempt loop
        let mut messages = vec![Message::system(&system_prompt), Message::user(problem)];

        let mut last_errors = String::new();

        for attempt in 1..=self.max_retries {
            // Call LLM
            let response = self.llm.complete(&messages, &self.llm_config)?;

            // Extract JSON
            let json_str = match extract_json(&response) {
                Some(j) => j.to_string(),
                None => {
                    if attempt < self.max_retries {
                        messages.push(Message::assistant(&response));
                        messages.push(Message::user(
                            "Your response did not contain valid JSON. Please respond with ONLY a JSON code block containing the CompositionGraph.",
                        ));
                        continue;
                    }
                    return Err(AgentError::NoJsonInResponse);
                }
            };

            // Parse graph
            let graph = match parse_graph(&json_str) {
                Ok(g) => g,
                Err(e) => {
                    if attempt < self.max_retries {
                        messages.push(Message::assistant(&response));
                        messages.push(Message::user(format!(
                            "The JSON was not a valid CompositionGraph: {e}. Please fix and try again."
                        )));
                        continue;
                    }
                    return Err(AgentError::InvalidGraph(e.to_string()));
                }
            };

            // Type check
            match check_graph(&graph.root, store) {
                Ok(_) => {
                    return Ok(ComposeResult {
                        graph,
                        attempts: attempt,
                    })
                }
                Err(errors) => {
                    last_errors = errors
                        .iter()
                        .map(|e| format!("{e}"))
                        .collect::<Vec<_>>()
                        .join("; ");
                    if attempt < self.max_retries {
                        messages.push(Message::assistant(&response));
                        messages.push(Message::user(format!(
                            "The composition graph has type errors:\n{last_errors}\n\nPlease fix the graph and try again."
                        )));
                        continue;
                    }
                }
            }
        }

        Err(AgentError::TypeCheckFailed {
            attempts: self.max_retries,
            errors: last_errors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::embedding::MockEmbeddingProvider;
    use crate::index::IndexConfig;
    use crate::llm::MockLlmProvider;
    use noether_core::stdlib::load_stdlib;
    use noether_store::{MemoryStore, StageStore};

    fn test_setup() -> (MemoryStore, SemanticIndex) {
        let mut store = MemoryStore::new();
        for stage in load_stdlib() {
            store.put(stage).unwrap();
        }
        let index = SemanticIndex::build(
            &store,
            Box::new(MockEmbeddingProvider::new(128)),
            IndexConfig::default(),
        )
        .unwrap();
        (store, index)
    }

    fn find_stage_id(store: &MemoryStore, desc_contains: &str) -> String {
        store
            .list(None)
            .into_iter()
            .find(|s| s.description.contains(desc_contains))
            .unwrap()
            .id
            .0
            .clone()
    }

    #[test]
    fn compose_with_valid_mock_response() {
        let (store, index) = test_setup();
        let to_text_id = find_stage_id(&store, "Convert any value to its text");

        // Mock LLM returns a valid single-stage graph
        let mock_response = format!(
            "```json\n{}\n```",
            serde_json::json!({
                "description": "convert to text",
                "version": "0.1.0",
                "root": {
                    "op": "Stage",
                    "id": to_text_id
                }
            })
        );

        let llm = MockLlmProvider::new(mock_response);
        let agent = CompositionAgent::new(&index, &llm, LlmConfig::default(), 3);
        let result = agent.compose("convert input to text", &store).unwrap();
        assert_eq!(result.attempts, 1);
        assert_eq!(result.graph.description, "convert to text");
    }

    #[test]
    fn compose_with_valid_sequential() {
        let (store, index) = test_setup();
        let to_json_id = find_stage_id(&store, "Serialize any value to a JSON");
        let parse_json_id = find_stage_id(&store, "Parse a JSON string");

        let mock_response = format!(
            "```json\n{}\n```",
            serde_json::json!({
                "description": "round-trip JSON",
                "version": "0.1.0",
                "root": {
                    "op": "Sequential",
                    "stages": [
                        {"op": "Stage", "id": to_json_id},
                        {"op": "Stage", "id": parse_json_id}
                    ]
                }
            })
        );

        let llm = MockLlmProvider::new(mock_response);
        let agent = CompositionAgent::new(&index, &llm, LlmConfig::default(), 3);
        let result = agent.compose("serialize and parse JSON", &store).unwrap();
        assert_eq!(result.attempts, 1);
    }

    #[test]
    fn compose_fails_with_no_json() {
        let (store, index) = test_setup();
        let llm = MockLlmProvider::new("I don't know how to help with that.");
        let agent = CompositionAgent::new(&index, &llm, LlmConfig::default(), 1);
        let result = agent.compose("do something", &store);
        assert!(result.is_err());
    }

    #[test]
    fn compose_fails_with_invalid_stage_id() {
        let (store, index) = test_setup();
        let mock_response = "```json\n{\"description\": \"test\", \"version\": \"0.1.0\", \"root\": {\"op\": \"Stage\", \"id\": \"nonexistent\"}}\n```";
        let llm = MockLlmProvider::new(mock_response);
        let agent = CompositionAgent::new(&index, &llm, LlmConfig::default(), 1);
        let result = agent.compose("test", &store);
        assert!(result.is_err());
    }
}
