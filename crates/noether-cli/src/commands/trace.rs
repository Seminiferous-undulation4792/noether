use crate::output::{acli_error, acli_ok};
use noether_engine::trace::JsonFileTraceStore;

pub fn cmd_trace(trace_store: &JsonFileTraceStore, composition_id: &str) {
    match trace_store.get(composition_id) {
        Some(trace) => {
            let json = serde_json::to_value(trace).unwrap();
            println!("{}", acli_ok(json));
        }
        None => {
            eprintln!(
                "{}",
                acli_error(&format!("trace {composition_id} not found"))
            );
            std::process::exit(1);
        }
    }
}
