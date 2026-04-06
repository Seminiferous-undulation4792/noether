use crate::output::acli_ok;
use noether_store::StageStore;
use serde_json::json;

pub fn cmd_stats(store: &impl StageStore) {
    let stats = store.stats();
    println!(
        "{}",
        acli_ok(json!({
            "total": stats.total,
            "by_lifecycle": stats.by_lifecycle,
            "by_effect": stats.by_effect,
        }))
    );
}
