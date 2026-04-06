use crate::effects::{Effect, EffectSet};
use crate::stage::{Stage, StageBuilder};
use crate::types::NType;
use ed25519_dalek::SigningKey;
use serde_json::json;

pub fn stages(key: &SigningKey) -> Vec<Stage> {
    vec![
        StageBuilder::new("to_text")
            .input(NType::Any)
            .output(NType::Text)
            .pure()
            .description("Convert any value to its text representation")
            .example(json!(42), json!("42"))
            .example(json!(true), json!("true"))
            .example(json!(null), json!("null"))
            .example(json!("hello"), json!("hello"))
            .example(json!([1, 2, 3]), json!("[1,2,3]"))
            .build_stdlib(key)
            .unwrap(),
        StageBuilder::new("to_number")
            .input(NType::union(vec![NType::Text, NType::Number, NType::Bool]))
            .output(NType::Number)
            .effects(EffectSet::new([Effect::Pure, Effect::Fallible]))
            .description("Parse a value as a number; fails on non-numeric text")
            .example(json!("42"), json!(42))
            .example(json!("9.81"), json!(9.81))
            .example(json!(true), json!(1))
            .example(json!(false), json!(0))
            .example(json!(100), json!(100))
            .build_stdlib(key)
            .unwrap(),
        StageBuilder::new("to_bool")
            .input(NType::union(vec![
                NType::Text,
                NType::Number,
                NType::Bool,
                NType::Null,
            ]))
            .output(NType::Bool)
            .pure()
            .description("Convert a value to boolean using truthiness rules")
            .example(json!(true), json!(true))
            .example(json!(false), json!(false))
            .example(json!(0), json!(false))
            .example(json!(1), json!(true))
            .example(json!(null), json!(false))
            .build_stdlib(key)
            .unwrap(),
        StageBuilder::new("parse_json")
            .input(NType::Text)
            .output(NType::Any)
            .effects(EffectSet::new([Effect::Pure, Effect::Fallible]))
            .description("Parse a JSON string into a structured value")
            .example(json!("42"), json!(42))
            .example(json!(r#"{"a":1}"#), json!({"a": 1}))
            .example(json!("[1,2,3]"), json!([1, 2, 3]))
            .example(json!("true"), json!(true))
            .example(json!("null"), json!(null))
            .build_stdlib(key)
            .unwrap(),
        StageBuilder::new("to_json")
            .input(NType::Any)
            .output(NType::Text)
            .pure()
            .description("Serialize any value to a JSON string")
            .example(json!(42), json!("42"))
            .example(json!({"a": 1}), json!(r#"{"a":1}"#))
            .example(json!([1, 2, 3]), json!("[1,2,3]"))
            .example(json!(true), json!("true"))
            .example(json!(null), json!("null"))
            .build_stdlib(key)
            .unwrap(),
    ]
}
