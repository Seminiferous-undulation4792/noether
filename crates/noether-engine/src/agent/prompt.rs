use crate::index::SearchResult;
use noether_core::stage::Stage;

/// Build the system prompt for the Composition Agent.
pub fn build_system_prompt(candidates: &[(&SearchResult, &Stage)]) -> String {
    let mut prompt = String::new();

    prompt.push_str(
        "You are Noether's Composition Agent. Your sole purpose is to translate problem \
         descriptions into valid composition graphs in Lagrange JSON format.\n\n",
    );

    prompt.push_str("## Rules\n");
    prompt.push_str(
        "1. ONLY use stages from the AVAILABLE STAGES list below. Never invent stage IDs.\n",
    );
    prompt.push_str("2. Output ONLY a valid JSON object wrapped in ```json blocks.\n");
    prompt.push_str(
        "3. The JSON must be a CompositionGraph with fields: description, version, root.\n",
    );
    prompt.push_str("4. Ensure types are compatible: output of one stage must be subtype of the next stage's input.\n\n");

    prompt.push_str("## Composition Operators\n\n");
    prompt.push_str("### Stage (leaf node)\n");
    prompt.push_str("```json\n{\"op\": \"Stage\", \"id\": \"<stage_hash>\"}\n```\n\n");

    prompt.push_str("### Sequential (A >> B >> C)\n");
    prompt.push_str(
        "```json\n{\"op\": \"Sequential\", \"stages\": [<node_A>, <node_B>, <node_C>]}\n```\n\n",
    );

    prompt.push_str("### Parallel (concurrent, merge outputs into Record)\n");
    prompt.push_str("```json\n{\"op\": \"Parallel\", \"branches\": {\"key1\": <node_A>, \"key2\": <node_B>}}\n```\n\n");

    prompt.push_str("### Branch (conditional)\n");
    prompt.push_str("```json\n{\"op\": \"Branch\", \"predicate\": <node>, \"if_true\": <node>, \"if_false\": <node>}\n```\n\n");

    prompt.push_str("### Fanout (one source to many targets)\n");
    prompt.push_str(
        "```json\n{\"op\": \"Fanout\", \"source\": <node>, \"targets\": [<node>, <node>]}\n```\n\n",
    );

    prompt.push_str("### Retry (retry on failure)\n");
    prompt.push_str("```json\n{\"op\": \"Retry\", \"stage\": <node>, \"max_attempts\": 3, \"delay_ms\": 500}\n```\n\n");

    prompt.push_str("## Available Stages\n\n");
    prompt.push_str("| ID | Description | Signature |\n");
    prompt.push_str("|---|---|---|\n");

    for (result, stage) in candidates {
        prompt.push_str(&format!(
            "| {} | {} | {} → {} |\n",
            stage.id.0, stage.description, stage.signature.input, stage.signature.output,
        ));
        let _ = result; // score is available but not shown in prompt
    }

    prompt.push_str("\n## Output Format\n\n");
    prompt.push_str("Respond with ONLY a JSON code block:\n");
    prompt.push_str("```json\n");
    prompt.push_str("{\n");
    prompt.push_str("  \"description\": \"<what this composition does>\",\n");
    prompt.push_str("  \"version\": \"0.1.0\",\n");
    prompt.push_str("  \"root\": { ... }\n");
    prompt.push_str("}\n");
    prompt.push_str("```\n");

    prompt
}

/// Extract JSON from an LLM response that may contain markdown code blocks.
pub fn extract_json(response: &str) -> Option<&str> {
    // Try to find ```json ... ``` block
    if let Some(start) = response.find("```json") {
        let json_start = start + 7; // skip ```json
        let json_content = &response[json_start..];
        if let Some(end) = json_content.find("```") {
            return Some(json_content[..end].trim());
        }
    }

    // Try to find ``` ... ``` block
    if let Some(start) = response.find("```") {
        let content_start = start + 3;
        let content = &response[content_start..];
        // Skip optional language tag on same line
        let json_start = content.find('\n').map(|n| n + 1).unwrap_or(0);
        let json_content = &content[json_start..];
        if let Some(end) = json_content.find("```") {
            return Some(json_content[..end].trim());
        }
    }

    // Try raw JSON (starts with {)
    let trimmed = response.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_from_code_block() {
        let response = "Here's the graph:\n```json\n{\"test\": true}\n```\nDone.";
        assert_eq!(extract_json(response), Some("{\"test\": true}"));
    }

    #[test]
    fn extract_json_from_plain_block() {
        let response = "```\n{\"test\": true}\n```";
        assert_eq!(extract_json(response), Some("{\"test\": true}"));
    }

    #[test]
    fn extract_raw_json() {
        let response = "{\"test\": true}";
        assert_eq!(extract_json(response), Some("{\"test\": true}"));
    }

    #[test]
    fn extract_json_none_for_text() {
        let response = "No JSON here, just text.";
        assert_eq!(extract_json(response), None);
    }

    #[test]
    fn extract_json_with_whitespace() {
        let response = "  \n```json\n  {\"a\": 1}  \n```\n  ";
        assert_eq!(extract_json(response), Some("{\"a\": 1}"));
    }
}
