use crate::output::{acli_error, acli_ok};
use noether_core::stage::{Stage, StageId};
use noether_engine::index::SemanticIndex;
use noether_store::{StageStore, StoreError};
use serde_json::json;
use std::fs;

pub fn cmd_add(store: &mut impl StageStore, spec_path: &str) {
    let content = match fs::read_to_string(spec_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", acli_error(&format!("failed to read file: {e}")));
            std::process::exit(1);
        }
    };

    let stage: Stage = match serde_json::from_str(&content) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", acli_error(&format!("invalid stage JSON: {e}")));
            std::process::exit(1);
        }
    };

    match store.put(stage) {
        Ok(id) => println!("{}", acli_ok(json!({"id": id.0}))),
        Err(StoreError::AlreadyExists(id)) => {
            println!("{}", acli_ok(json!({"id": id.0, "note": "already exists"})));
        }
        Err(e) => {
            eprintln!("{}", acli_error(&format!("{e}")));
            std::process::exit(1);
        }
    }
}

pub fn cmd_get(store: &impl StageStore, hash: &str) {
    let id = StageId(hash.into());
    match store.get(&id) {
        Ok(Some(stage)) => {
            let json = serde_json::to_value(stage).unwrap();
            println!("{}", acli_ok(json));
        }
        Ok(None) => {
            eprintln!("{}", acli_error(&format!("stage {hash} not found")));
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("{}", acli_error(&format!("{e}")));
            std::process::exit(1);
        }
    }
}

pub fn cmd_list(store: &impl StageStore) {
    let stages = store.list(None);
    let mut sorted: Vec<&Stage> = stages;
    sorted.sort_by(|a, b| a.description.cmp(&b.description));

    let entries: Vec<serde_json::Value> = sorted
        .iter()
        .map(|s| {
            json!({
                "id": &s.id.0[..8.min(s.id.0.len())],
                "description": s.description,
                "signature": format!("{} → {}", s.signature.input, s.signature.output),
                "lifecycle": format!("{:?}", s.lifecycle),
            })
        })
        .collect();

    println!(
        "{}",
        acli_ok(json!({"stages": entries, "count": entries.len()}))
    );
}

pub fn cmd_search(store: &impl StageStore, index: &SemanticIndex, query: &str) {
    let results = match index.search(query, 20) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", acli_error(&format!("search failed: {e}")));
            std::process::exit(1);
        }
    };

    let entries: Vec<serde_json::Value> = results
        .iter()
        .filter_map(|r| {
            let stage = store.get(&r.stage_id).ok()??;
            Some(json!({
                "id": &stage.id.0[..8.min(stage.id.0.len())],
                "description": stage.description,
                "signature": format!("{} → {}", stage.signature.input, stage.signature.output),
                "score": format!("{:.3}", r.score),
                "scores": {
                    "signature": format!("{:.3}", r.signature_score),
                    "semantic": format!("{:.3}", r.semantic_score),
                    "example": format!("{:.3}", r.example_score),
                }
            }))
        })
        .collect();

    println!(
        "{}",
        acli_ok(json!({"query": query, "results": entries, "count": entries.len()}))
    );
}
