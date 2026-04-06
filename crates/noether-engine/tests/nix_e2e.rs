
#[test]
fn nix_e2e_word_count() {
    use noether_core::stage::{StageBuilder, StageLifecycle};
    use noether_core::types::NType;
    use noether_engine::executor::composite::CompositeExecutor;
    use noether_engine::executor::StageExecutor;
    use noether_store::{MemoryStore, StageStore};
    use serde_json::json;

    // Build Python implementation of word_count
    let python_code = r#"
def execute(input_value):
    text = input_value.get("text", "")
    words = text.split()
    return {"count": len(words), "words": words}
"#;

    let impl_hash = {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(python_code.as_bytes()))
    };

    let stage = StageBuilder::new("word_count")
        .input(NType::Record(
            [("text".into(), NType::Text)]
                .into_iter()
                .collect::<std::collections::BTreeMap<_, _>>()
                .into(),
        ))
        .output(NType::Any)
        .description("Count words in a text string")
        .example(
            json!({"text": "hello world"}),
            json!({"count": 2, "words": ["hello", "world"]}),
        )
        .example(
            json!({"text": "one two three"}),
            json!({"count": 3, "words": ["one", "two", "three"]}),
        )
        .example(
            json!({"text": "a b c d e"}),
            json!({"count": 5, "words": ["a", "b", "c", "d", "e"]}),
        )
        .implementation_code(python_code, "python")
        .build_unsigned(impl_hash)
        .unwrap();

    let stage_id = stage.id.clone();

    let mut store = MemoryStore::new();
    let _ = store.put(stage);
    let _ = store.update_lifecycle(&stage_id, StageLifecycle::Active);

    let executor = CompositeExecutor::from_store(&store);

    if !executor.nix_available() {
        eprintln!("nix not available, skipping NixExecutor path");
        return;
    }

    let input = json!({"text": "the quick brown fox jumps over the lazy dog"});
    let result = executor.execute(&stage_id, &input).expect("execution failed");

    let count = result["count"].as_u64().expect("count should be a number");
    assert_eq!(count, 9, "expected 9 words");

    let words = result["words"].as_array().expect("words should be an array");
    assert_eq!(words.len(), 9);
    assert_eq!(words[0], json!("the"));

    println!("word_count result: {result}");
}
