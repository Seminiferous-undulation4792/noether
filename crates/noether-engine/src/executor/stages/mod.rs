pub mod collections;
pub mod data;
pub mod scalar;
pub mod text;

use super::ExecutionError;
use serde_json::Value;

/// A stage implementation function.
pub type StageFn = fn(&Value) -> Result<Value, ExecutionError>;

/// Find the implementation for a stage by matching its description.
/// Returns None for stages without real implementations (I/O, LLM, control, internal).
pub fn find_implementation(description: &str) -> Option<StageFn> {
    // Scalar
    match description {
        "Convert any value to its text representation" => Some(scalar::to_text),
        "Parse a value as a number; fails on non-numeric text" => Some(scalar::to_number),
        "Convert a value to boolean using truthiness rules" => Some(scalar::to_bool),
        "Parse a JSON string into a structured value" => Some(scalar::parse_json),
        "Serialize any value to a JSON string" => Some(scalar::to_json),

        // Text
        "Split text by a delimiter into a list of strings" => Some(text::text_split),
        "Join a list of strings with a delimiter" => Some(text::text_join),
        "Match text against a regex pattern; fails on invalid regex" => Some(text::regex_match),
        "Replace regex matches in text; fails on invalid regex" => Some(text::regex_replace),
        "Interpolate variables into a template string using {{key}} syntax" => {
            Some(text::text_template)
        }
        "Compute a cryptographic hash of text; defaults to SHA-256" => Some(text::text_hash),

        // Collections
        "Sort a list; optionally by a field name and/or in descending order" => {
            Some(collections::sort)
        }
        "Flatten a list of lists into a single list" => Some(collections::flatten),
        "Combine two lists into a list of pairs, truncating to the shorter list" => {
            Some(collections::zip)
        }
        "Take the first N elements from a list" => Some(collections::take),
        "Group list items by the value of a named field" => Some(collections::group_by),

        // Data
        "Deep merge two JSON values; patch values override base" => Some(data::json_merge),
        "Extract a value from JSON data using a JSONPath expression" => Some(data::json_path),
        "Validate data against a JSON schema; returns validation results" => {
            Some(data::json_schema_validate)
        }

        _ => None,
    }
}
