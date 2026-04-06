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

    // --- Few-shot example using real IDs when available ---
    // Look for parse_json and to_json in candidates so the example uses actual hashes.
    let parse_json_id = find_candidate_id(candidates, "Parse a JSON string");
    let to_json_id = find_candidate_id(candidates, "Serialize any value to a JSON");

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
    prompt.push_str(&format!(
        "      {{\"op\": \"Stage\", \"id\": \"{}\"}},\n",
        parse_json_id
    ));
    prompt.push_str(&format!(
        "      {{\"op\": \"Stage\", \"id\": \"{}\"}}\n",
        to_json_id
    ));
    prompt.push_str("    ]\n");
    prompt.push_str("  }\n");
    prompt.push_str("}\n");
    prompt.push_str("```\n\n");

    // --- Available stages with examples, ordered by relevance score ---
    prompt.push_str("## Available Stages\n\n");
    prompt.push_str("Stages are listed by relevance to your problem (highest first).\n\n");

    for (result, stage) in candidates {
        prompt.push_str(&format!(
            "### `{}` — {} _(relevance: {:.2})_\n",
            stage.id.0, stage.description, result.score
        ));
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

/// Search `candidates` for a stage whose description contains `needle`
/// and return its ID. Falls back to `<needle>` as a labelled placeholder
/// so the few-shot example is always syntactically valid JSON.
fn find_candidate_id(candidates: &[(&SearchResult, &Stage)], needle: &str) -> String {
    candidates
        .iter()
        .find(|(_, s)| s.description.contains(needle))
        .map(|(_, s)| s.id.0.clone())
        .unwrap_or_else(|| format!("<{needle}>"))
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
    use crate::index::SearchResult;
    use noether_core::stage::StageId;

    fn make_search_result(id: &str, score: f32) -> SearchResult {
        SearchResult {
            stage_id: StageId(id.into()),
            score,
            signature_score: score,
            semantic_score: score,
            example_score: score,
        }
    }

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

    #[test]
    fn few_shot_uses_real_ids_when_candidates_present() {
        use noether_core::stdlib::load_stdlib;

        let stages = load_stdlib();
        let parse_json = stages
            .iter()
            .find(|s| s.description.contains("Parse a JSON string"))
            .unwrap();
        let to_json = stages
            .iter()
            .find(|s| s.description.contains("Serialize any value to a JSON"))
            .unwrap();

        let r1 = make_search_result(&parse_json.id.0, 0.9);
        let r2 = make_search_result(&to_json.id.0, 0.8);
        let candidates: Vec<(&SearchResult, &Stage)> = vec![(&r1, parse_json), (&r2, to_json)];

        let prompt = build_system_prompt(&candidates);

        // The few-shot example must contain the real hashes, not placeholders.
        assert!(
            prompt.contains(&parse_json.id.0),
            "prompt should contain real parse_json hash"
        );
        assert!(
            prompt.contains(&to_json.id.0),
            "prompt should contain real to_json hash"
        );
        assert!(
            !prompt.contains("PARSE_JSON_ID") && !prompt.contains("TO_JSON_ID"),
            "prompt must not contain placeholder IDs"
        );
    }

    #[test]
    fn few_shot_falls_back_to_placeholder_when_stages_absent() {
        let prompt = build_system_prompt(&[]);
        // With no candidates the fallback label appears (angle-bracket wrapped needle text).
        assert!(
            prompt.contains("<Parse a JSON string>"),
            "expected placeholder when parse_json not in candidates"
        );
    }

    #[test]
    fn candidates_show_relevance_score() {
        use noether_core::stdlib::load_stdlib;

        let stages = load_stdlib();
        let stage = stages.first().unwrap();
        let r = make_search_result(&stage.id.0, 0.75);
        let candidates: Vec<(&SearchResult, &Stage)> = vec![(&r, stage)];

        let prompt = build_system_prompt(&candidates);
        assert!(
            prompt.contains("relevance: 0.75"),
            "prompt should display the fused relevance score"
        );
    }
}
