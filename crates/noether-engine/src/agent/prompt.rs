use crate::index::SearchResult;
use noether_core::stage::Stage;
use noether_core::types::NType;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Synthesis types ────────────────────────────────────────────────────────

/// Specification for a stage the Composition Agent wants synthesized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisSpec {
    pub name: String,
    pub description: String,
    pub input: NType,
    pub output: NType,
    pub rationale: String,
}

/// Code + examples returned by the synthesis codegen LLM call.
#[derive(Debug, Clone, Deserialize)]
pub struct SynthesisResponse {
    pub examples: Vec<SynthesisExample>,
    pub implementation: String,
    #[serde(default = "default_language")]
    pub language: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SynthesisExample {
    pub input: Value,
    pub output: Value,
}

fn default_language() -> String {
    "python".into()
}

// ── Prompt builders ────────────────────────────────────────────────────────
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

    // --- Synthesis option (last resort) ---
    prompt.push_str("## If You Need a New Stage\n\n");
    prompt.push_str("If—and only if—no combination of existing stages can solve the problem, output a synthesis request instead of a graph:\n\n");
    prompt.push_str("```json\n");
    prompt.push_str("{\n");
    prompt.push_str("  \"action\": \"synthesize\",\n");
    prompt.push_str("  \"spec\": {\n");
    prompt.push_str("    \"name\": \"snake_case_stage_name\",\n");
    prompt.push_str("    \"description\": \"One-sentence description of what this stage does\",\n");
    prompt.push_str("    \"input\": {\"kind\": \"Text\"},\n");
    prompt.push_str("    \"output\": {\"kind\": \"Number\"},\n");
    prompt.push_str("    \"rationale\": \"Why no available stage satisfies this\"\n");
    prompt.push_str("  }\n");
    prompt.push_str("}\n");
    prompt.push_str("```\n\n");
    prompt.push_str("NType JSON format: `{\"kind\":\"Text\"}`, `{\"kind\":\"Number\"}`, `{\"kind\":\"Bool\"}`, `{\"kind\":\"Any\"}`, `{\"kind\":\"Null\"}`, ");
    prompt.push_str("`{\"kind\":\"List\",\"value\":<T>}`, `{\"kind\":\"Record\",\"value\":{\"field\":<T>,...}}`, `{\"kind\":\"Union\",\"value\":[<T>,...]}` \n\n");
    prompt.push_str("**Always prefer composing existing stages. Only use synthesis if composition is genuinely impossible.**\n\n");

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

/// Build the codegen prompt that asks the LLM to implement a synthesized stage.
pub fn build_synthesis_prompt(spec: &SynthesisSpec) -> String {
    let mut p = String::new();
    p.push_str(
        "You are generating a stage implementation for the Noether composition platform.\n\n",
    );
    p.push_str("## Stage Specification\n\n");
    p.push_str(&format!("- **Name**: `{}`\n", spec.name));
    p.push_str(&format!("- **Description**: {}\n", spec.description));
    p.push_str(&format!("- **Input type**: `{}`\n", spec.input));
    p.push_str(&format!("- **Output type**: `{}`\n\n", spec.output));

    p.push_str("## Your Task\n\n");
    p.push_str(
        "1. Produce at least 3 concrete input/output example pairs matching the type signature.\n",
    );
    p.push_str("2. Write a Python function `execute(input_value)` that implements this stage.\n");
    p.push_str(
        "   `input_value` is a Python dict/str/number/list/bool/None matching the input type.\n",
    );
    p.push_str("   Return a value matching the output type.\n\n");

    p.push_str("## Output Format\n\n");
    p.push_str("Respond with ONLY this JSON (no other text):\n");
    p.push_str("```json\n");
    p.push_str("{\n");
    p.push_str("  \"examples\": [\n");
    p.push_str("    {\"input\": <value>, \"output\": <value>},\n");
    p.push_str("    {\"input\": <value>, \"output\": <value>},\n");
    p.push_str("    {\"input\": <value>, \"output\": <value>}\n");
    p.push_str("  ],\n");
    p.push_str("  \"implementation\": \"def execute(input_value):\\n    ...\",\n");
    p.push_str("  \"language\": \"python\"\n");
    p.push_str("}\n");
    p.push_str("```\n");
    p
}

/// Try to parse a synthesis request from the LLM response.
/// Returns `Some(SynthesisSpec)` only when the JSON contains `"action": "synthesize"`.
pub fn extract_synthesis_spec(response: &str) -> Option<SynthesisSpec> {
    let json_str = extract_json(response)?;
    let v: serde_json::Value = serde_json::from_str(json_str).ok()?;
    if v.get("action").and_then(|a| a.as_str()) != Some("synthesize") {
        return None;
    }
    let spec = v.get("spec")?;
    serde_json::from_value(spec.clone()).ok()
}

/// Try to parse a synthesis response (examples + implementation) from the LLM.
pub fn extract_synthesis_response(response: &str) -> Option<SynthesisResponse> {
    let json_str = extract_json(response)?;
    serde_json::from_str(json_str).ok()
}

pub fn extract_json(response: &str) -> Option<&str> {
    // 1. Prefer ```json ... ``` fenced block
    if let Some(start) = response.find("```json") {
        let json_start = start + 7;
        let json_content = &response[json_start..];
        if let Some(end) = json_content.find("```") {
            return Some(json_content[..end].trim());
        }
    }

    // 2. Plain ``` ... ``` fenced block (skip language tag on first line if any)
    if let Some(start) = response.find("```") {
        let content_start = start + 3;
        let content = &response[content_start..];
        // Skip a non-brace first line (e.g. a language tag like "json" without the marker)
        let (skip, rest) = match content.find('\n') {
            Some(nl) => {
                let first_line = content[..nl].trim();
                if first_line.starts_with('{') {
                    (0, content)
                } else {
                    (nl + 1, &content[nl + 1..])
                }
            }
            None => (0, content),
        };
        let _ = skip;
        if let Some(end) = rest.find("```") {
            let candidate = rest[..end].trim();
            if candidate.starts_with('{') {
                return Some(candidate);
            }
        }
    }

    // 3. Raw JSON anywhere in the response: scan for the first top-level { ... } span
    // using brace depth counting (handles nested objects correctly).
    if let Some(brace_start) = response.find('{') {
        let bytes = response.as_bytes();
        let mut depth: i32 = 0;
        let mut in_string = false;
        let mut escape = false;
        let mut brace_end: Option<usize> = None;

        for (i, &b) in bytes[brace_start..].iter().enumerate() {
            if escape {
                escape = false;
                continue;
            }
            if in_string {
                match b {
                    b'\\' => escape = true,
                    b'"' => in_string = false,
                    _ => {}
                }
                continue;
            }
            match b {
                b'"' => in_string = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        brace_end = Some(brace_start + i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }

        if let Some(end) = brace_end {
            let candidate = response[brace_start..end].trim();
            if !candidate.is_empty() {
                return Some(candidate);
            }
        }
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
    fn extract_synthesis_spec_parses_valid_request() {
        let input_json = serde_json::to_string(&NType::Text).unwrap();
        let output_json = serde_json::to_string(&NType::Number).unwrap();
        let response = format!(
            "```json\n{}\n```",
            serde_json::json!({
                "action": "synthesize",
                "spec": {
                    "name": "count_words",
                    "description": "Count the number of words in a text",
                    "input": serde_json::from_str::<serde_json::Value>(&input_json).unwrap(),
                    "output": serde_json::from_str::<serde_json::Value>(&output_json).unwrap(),
                    "rationale": "No existing stage counts words"
                }
            })
        );
        let spec = extract_synthesis_spec(&response).unwrap();
        assert_eq!(spec.name, "count_words");
        assert_eq!(spec.input, NType::Text);
        assert_eq!(spec.output, NType::Number);
    }

    #[test]
    fn extract_synthesis_spec_returns_none_for_composition_graph() {
        let response = "```json\n{\"description\":\"test\",\"version\":\"0.1.0\",\"root\":{\"op\":\"Stage\",\"id\":\"abc\"}}\n```";
        assert!(extract_synthesis_spec(response).is_none());
    }

    #[test]
    fn extract_synthesis_response_parses_examples_and_code() {
        let response = "```json\n{\"examples\":[{\"input\":\"hello world\",\"output\":2},{\"input\":\"foo\",\"output\":1}],\"implementation\":\"def execute(v): return len(v.split())\",\"language\":\"python\"}\n```";
        let resp = extract_synthesis_response(response).unwrap();
        assert_eq!(resp.examples.len(), 2);
        assert_eq!(resp.language, "python");
        assert!(resp.implementation.contains("execute"));
    }

    #[test]
    fn build_synthesis_prompt_contains_spec_fields() {
        let spec = SynthesisSpec {
            name: "reverse_text".into(),
            description: "Reverse a string".into(),
            input: NType::Text,
            output: NType::Text,
            rationale: "no existing stage reverses text".into(),
        };
        let prompt = build_synthesis_prompt(&spec);
        assert!(prompt.contains("reverse_text"));
        assert!(prompt.contains("Reverse a string"));
        assert!(prompt.contains("execute(input_value)"));
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
