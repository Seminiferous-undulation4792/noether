use crate::index::SearchResult;
use noether_core::stage::Stage;

/// Build the system prompt for the Composition Agent.
pub fn build_system_prompt(candidates: &[(&SearchResult, &Stage)]) -> String {
    let mut prompt = String::new();

    // --- Role ---
    prompt.push_str(
        "You are Noether's Composition Agent. You translate problem descriptions into \
         composition graphs in Lagrange JSON format.\n\n",
    );

    // --- Critical rules ---
    prompt.push_str("## CRITICAL RULES\n\n");
    prompt.push_str("1. ONLY use stage IDs from the AVAILABLE STAGES list. Never invent IDs.\n");
    prompt.push_str("2. Types MUST match: the output type of one stage must be a subtype of the next stage's input type.\n");
    prompt.push_str("3. Most stages take Record inputs with SPECIFIC FIELD NAMES. Check the examples carefully.\n");
    prompt.push_str("4. Output ONLY a JSON code block — no explanation before or after.\n\n");

    // --- Type system primer ---
    prompt.push_str("## Type System\n\n");
    prompt
        .push_str("- `Any` accepts any value. `Text`, `Number`, `Bool`, `Null` are primitives.\n");
    prompt.push_str("- `Record { field: Type }` is an object with named fields. The stage REQUIRES exactly those fields.\n");
    prompt.push_str("- `List<T>` is an array. `Map<K,V>` is a key-value object.\n");
    prompt.push_str("- `T | Null` means the field is optional (can be null).\n");
    prompt.push_str(
        "- Width subtyping: `{a, b, c}` is subtype of `{a, b}` — extra fields are OK.\n\n",
    );

    // --- Operators (concise) ---
    prompt.push_str("## Operators\n\n");
    prompt.push_str("- **Stage**: `{\"op\": \"Stage\", \"id\": \"<hash>\"}`\n");
    prompt.push_str("- **Sequential**: `{\"op\": \"Sequential\", \"stages\": [A, B, C]}` — output of A feeds B, then C\n");
    prompt.push_str("- **Parallel**: `{\"op\": \"Parallel\", \"branches\": {\"key1\": A, \"key2\": B}}` — concurrent, merges to Record\n");
    prompt.push_str(
        "- **Branch**: `{\"op\": \"Branch\", \"predicate\": P, \"if_true\": A, \"if_false\": B}`\n",
    );
    prompt.push_str("- **Fanout**: `{\"op\": \"Fanout\", \"source\": A, \"targets\": [B, C]}`\n");
    prompt.push_str("- **Retry**: `{\"op\": \"Retry\", \"stage\": A, \"max_attempts\": 3, \"delay_ms\": 500}`\n\n");

    // --- Few-shot example ---
    prompt.push_str("## EXAMPLE: Multi-stage composition\n\n");
    prompt.push_str("Problem: \"Parse a JSON string and serialize it back\"\n\n");
    prompt.push_str("The stage `parse_json` has input `Text` and output `Any`.\n");
    prompt.push_str("The stage `to_json` has input `Any` and output `Text`.\n");
    prompt.push_str("Since `Any` (output of parse_json) is subtype of `Any` (input of to_json), they compose.\n\n");
    prompt.push_str("```json\n");
    prompt.push_str("{\n");
    prompt.push_str("  \"description\": \"Parse JSON then serialize back to text\",\n");
    prompt.push_str("  \"version\": \"0.1.0\",\n");
    prompt.push_str("  \"root\": {\n");
    prompt.push_str("    \"op\": \"Sequential\",\n");
    prompt.push_str("    \"stages\": [\n");
    prompt.push_str("      {\"op\": \"Stage\", \"id\": \"PARSE_JSON_ID\"},\n");
    prompt.push_str("      {\"op\": \"Stage\", \"id\": \"TO_JSON_ID\"}\n");
    prompt.push_str("    ]\n");
    prompt.push_str("  }\n");
    prompt.push_str("}\n");
    prompt.push_str("```\n\n");

    // --- Available stages with examples ---
    prompt.push_str("## Available Stages\n\n");

    for (_result, stage) in candidates {
        prompt.push_str(&format!("### `{}` — {}\n", stage.id.0, stage.description));
        prompt.push_str(&format!(
            "- **Input**: `{}`\n- **Output**: `{}`\n",
            stage.signature.input, stage.signature.output,
        ));

        // Show first 2 examples with concrete data
        for ex in stage.examples.iter().take(2) {
            let input_str = serde_json::to_string(&ex.input).unwrap_or_default();
            let output_str = serde_json::to_string(&ex.output).unwrap_or_default();
            prompt.push_str(&format!("- Example: `{input_str}` → `{output_str}`\n"));
        }
        prompt.push('\n');
    }

    // --- Output format ---
    prompt.push_str("## Your Response\n\n");
    prompt.push_str("Respond with ONLY this JSON (no other text):\n");
    prompt.push_str("```json\n");
    prompt.push_str("{\n");
    prompt.push_str("  \"description\": \"<what this composition does>\",\n");
    prompt.push_str("  \"version\": \"0.1.0\",\n");
    prompt.push_str("  \"root\": { <composition using operators above> }\n");
    prompt.push_str("}\n");
    prompt.push_str("```\n");

    prompt
}

/// Extract JSON from an LLM response that may contain markdown code blocks.
pub fn extract_json(response: &str) -> Option<&str> {
    // Try to find ```json ... ``` block
    if let Some(start) = response.find("```json") {
        let json_start = start + 7;
        let json_content = &response[json_start..];
        if let Some(end) = json_content.find("```") {
            return Some(json_content[..end].trim());
        }
    }

    // Try to find ``` ... ``` block
    if let Some(start) = response.find("```") {
        let content_start = start + 3;
        let content = &response[content_start..];
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
