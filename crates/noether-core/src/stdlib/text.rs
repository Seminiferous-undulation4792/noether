use crate::effects::{Effect, EffectSet};
use crate::stage::{Stage, StageBuilder};
use crate::types::NType;
use ed25519_dalek::SigningKey;
use serde_json::json;

pub fn stages(key: &SigningKey) -> Vec<Stage> {
    vec![
        StageBuilder::new("text_split")
            .input(NType::record([
                ("text", NType::Text),
                ("delimiter", NType::Text),
            ]))
            .output(NType::List(Box::new(NType::Text)))
            .pure()
            .description("Split text by a delimiter into a list of strings")
            .example(json!({"text": "a,b,c", "delimiter": ","}), json!(["a", "b", "c"]))
            .example(json!({"text": "hello world", "delimiter": " "}), json!(["hello", "world"]))
            .example(json!({"text": "one", "delimiter": ","}), json!(["one"]))
            .example(json!({"text": "", "delimiter": ","}), json!([""]))
            .example(json!({"text": "a::b::c", "delimiter": "::"}), json!(["a", "b", "c"]))
            .build_stdlib(key)
            .unwrap(),
        StageBuilder::new("text_join")
            .input(NType::record([
                ("items", NType::List(Box::new(NType::Text))),
                ("delimiter", NType::Text),
            ]))
            .output(NType::Text)
            .pure()
            .description("Join a list of strings with a delimiter")
            .example(json!({"items": ["a", "b", "c"], "delimiter": ","}), json!("a,b,c"))
            .example(json!({"items": ["hello", "world"], "delimiter": " "}), json!("hello world"))
            .example(json!({"items": ["one"], "delimiter": ","}), json!("one"))
            .example(json!({"items": [], "delimiter": ","}), json!(""))
            .example(json!({"items": ["a", "b"], "delimiter": ""}), json!("ab"))
            .build_stdlib(key)
            .unwrap(),
        StageBuilder::new("regex_match")
            .input(NType::record([
                ("text", NType::Text),
                ("pattern", NType::Text),
            ]))
            .output(NType::record([
                ("matched", NType::Bool),
                ("groups", NType::List(Box::new(NType::Text))),
                ("full_match", NType::optional(NType::Text)),
            ]))
            .effects(EffectSet::new([Effect::Pure, Effect::Fallible]))
            .description("Match text against a regex pattern; fails on invalid regex")
            .example(
                json!({"text": "hello123", "pattern": "(\\d+)"}),
                json!({"matched": true, "groups": ["123"], "full_match": "123"}),
            )
            .example(
                json!({"text": "abc", "pattern": "\\d+"}),
                json!({"matched": false, "groups": [], "full_match": null}),
            )
            .example(
                json!({"text": "2024-01-15", "pattern": "(\\d{4})-(\\d{2})-(\\d{2})"}),
                json!({"matched": true, "groups": ["2024", "01", "15"], "full_match": "2024-01-15"}),
            )
            .example(
                json!({"text": "test@email.com", "pattern": "(.+)@(.+)"}),
                json!({"matched": true, "groups": ["test", "email.com"], "full_match": "test@email.com"}),
            )
            .example(
                json!({"text": "no match", "pattern": "^\\d+$"}),
                json!({"matched": false, "groups": [], "full_match": null}),
            )
            .build_stdlib(key)
            .unwrap(),
        StageBuilder::new("regex_replace")
            .input(NType::record([
                ("text", NType::Text),
                ("pattern", NType::Text),
                ("replacement", NType::Text),
            ]))
            .output(NType::Text)
            .effects(EffectSet::new([Effect::Pure, Effect::Fallible]))
            .description("Replace regex matches in text; fails on invalid regex")
            .example(json!({"text": "hello 123 world", "pattern": "\\d+", "replacement": "NUM"}), json!("hello NUM world"))
            .example(json!({"text": "aaa", "pattern": "a", "replacement": "b"}), json!("bbb"))
            .example(json!({"text": "foo bar", "pattern": "\\s+", "replacement": "_"}), json!("foo_bar"))
            .example(json!({"text": "no match", "pattern": "\\d+", "replacement": "X"}), json!("no match"))
            .example(json!({"text": "abc", "pattern": "(.)", "replacement": "[$1]"}), json!("[a][b][c]"))
            .build_stdlib(key)
            .unwrap(),
        StageBuilder::new("text_template")
            .input(NType::record([
                ("template", NType::Text),
                ("variables", NType::Map {
                    key: Box::new(NType::Text),
                    value: Box::new(NType::Text),
                }),
            ]))
            .output(NType::Text)
            .pure()
            .description("Interpolate variables into a template string using {{key}} syntax")
            .example(json!({"template": "Hello, {{name}}!", "variables": {"name": "Alice"}}), json!("Hello, Alice!"))
            .example(json!({"template": "{{a}} + {{b}}", "variables": {"a": "1", "b": "2"}}), json!("1 + 2"))
            .example(json!({"template": "no vars", "variables": {}}), json!("no vars"))
            .example(json!({"template": "{{x}}", "variables": {"x": "value"}}), json!("value"))
            .example(json!({"template": "{{a}}{{a}}", "variables": {"a": "x"}}), json!("xx"))
            .build_stdlib(key)
            .unwrap(),
        StageBuilder::new("text_hash")
            .input(NType::record([
                ("text", NType::Text),
                ("algorithm", NType::optional(NType::Text)),
            ]))
            .output(NType::record([
                ("hash", NType::Text),
                ("algorithm", NType::Text),
            ]))
            .pure()
            .description("Compute a cryptographic hash of text; defaults to SHA-256")
            .example(json!({"text": "hello", "algorithm": "sha256"}), json!({"hash": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824", "algorithm": "sha256"}))
            .example(json!({"text": "hello", "algorithm": null}), json!({"hash": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824", "algorithm": "sha256"}))
            .example(json!({"text": "", "algorithm": "sha256"}), json!({"hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "algorithm": "sha256"}))
            .example(json!({"text": "test", "algorithm": "md5"}), json!({"hash": "098f6bcd4621d373cade4e832627b4f6", "algorithm": "md5"}))
            .example(json!({"text": "abc", "algorithm": "sha256"}), json!({"hash": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", "algorithm": "sha256"}))
            .build_stdlib(key)
            .unwrap(),
    ]
}
