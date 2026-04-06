use crate::output::{acli_error, acli_ok};
use noether_engine::agent::SynthesisResult;
use noether_engine::checker::check_graph;
use noether_engine::composition_cache::CompositionCache;
use noether_engine::executor::composite::CompositeExecutor;
use noether_engine::executor::runner::run_composition;
use noether_engine::index::SemanticIndex;
use noether_engine::lagrange::{compute_composition_id, serialize_graph, CompositionGraph};
use noether_engine::llm::{LlmConfig, LlmProvider};
use noether_engine::planner::plan_graph;
use noether_engine::providers;
use noether_store::StageStore;
use serde_json::json;
use std::path::Path;

pub struct ComposeOptions<'a> {
    pub model: &'a str,
    pub dry_run: bool,
    pub input: &'a serde_json::Value,
    pub force: bool,
    pub cache_path: &'a Path,
}

pub fn cmd_compose(
    store: &mut dyn StageStore,
    index: &mut SemanticIndex,
    llm: &dyn LlmProvider,
    problem: &str,
    opts: ComposeOptions<'_>,
) {
    let mut cache = CompositionCache::open(opts.cache_path);

    // ── Cache lookup ──────────────────────────────────────────────────────────
    if !opts.force {
        if let Some(cached) = cache.get(problem) {
            let age_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
                .saturating_sub(cached.cached_at);
            eprintln!(
                "Cache hit (model: {}, composed: {}s ago). Use --force to recompose.",
                cached.model, age_secs,
            );
            emit_result(store, EmitCtx {
                model: opts.model,
                dry_run: opts.dry_run,
                input: opts.input,
                from_cache: true,
                attempts: 0,
                synthesized: &[],
                graph: &cached.graph.clone(),
            });
            return;
        }
    }

    // ── LLM composition ───────────────────────────────────────────────────────
    let llm_config = LlmConfig {
        model: opts.model.into(),
        max_tokens: 4096,
        temperature: 0.2,
    };

    let mut agent = noether_engine::agent::CompositionAgent::new(index, llm, llm_config, 3);
    let result = match agent.compose(problem, store) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", acli_error(&format!("composition failed: {e}")));
            std::process::exit(2);
        }
    };

    // Cache only graphs that don't depend on freshly synthesized stage IDs.
    if result.synthesized.is_empty() {
        cache.insert(problem, result.graph.clone(), opts.model);
    }

    let (graph, synthesized, attempts) = (result.graph, result.synthesized, result.attempts);
    emit_result(store, EmitCtx {
        model: opts.model,
        dry_run: opts.dry_run,
        input: opts.input,
        from_cache: false,
        attempts,
        synthesized: &synthesized,
        graph: &graph,
    });
}

struct EmitCtx<'a> {
    model: &'a str,
    dry_run: bool,
    input: &'a serde_json::Value,
    from_cache: bool,
    attempts: u32,
    synthesized: &'a [SynthesisResult],
    graph: &'a CompositionGraph,
}

fn emit_result(store: &mut dyn StageStore, ctx: EmitCtx<'_>) {
    let composition_id = compute_composition_id(ctx.graph).unwrap_or_else(|_| "unknown".into());
    let graph_json = serialize_graph(ctx.graph).unwrap_or_else(|_| "{}".into());

    let synthesized_json: Vec<serde_json::Value> = ctx.synthesized
        .iter()
        .map(|s| {
            json!({
                "stage_id": s.stage_id.0,
                "language": s.language,
                "attempts": s.attempts,
                "is_new": s.is_new,
            })
        })
        .collect();

    let resolved = check_graph(&ctx.graph.root, store).ok();
    let plan = plan_graph(&ctx.graph.root, store);

    if ctx.dry_run {
        println!(
            "{}",
            acli_ok(json!({
                "mode": "dry-run",
                "composition_id": composition_id,
                "attempts": ctx.attempts,
                "from_cache": ctx.from_cache,
                "synthesized": synthesized_json,
                "graph": serde_json::from_str::<serde_json::Value>(&graph_json).unwrap_or(json!(null)),
                "type_check": resolved.as_ref().map(|r| json!({
                    "input": format!("{}", r.input),
                    "output": format!("{}", r.output),
                })),
                "plan": {
                    "steps": plan.steps.len(),
                    "parallel_groups": plan.parallel_groups.len(),
                    "cost": plan.cost,
                },
            }))
        );
        return;
    }

    let mut executor = CompositeExecutor::from_store(store).with_llm(
        providers::build_llm_provider().0,
        LlmConfig {
            model: ctx.model.into(),
            max_tokens: 4096,
            temperature: 0.2,
        },
    );
    for syn in ctx.synthesized {
        executor.register_synthesized(&syn.stage_id, &syn.implementation, &syn.language);
    }

    if !ctx.synthesized.is_empty() && !executor.nix_available() {
        eprintln!("Warning: synthesized stages will use fallback execution (nix not available).");
    }

    match run_composition(&ctx.graph.root, ctx.input, &executor, &composition_id) {
        Ok(exec_result) => {
            println!(
                "{}",
                acli_ok(json!({
                    "composition_id": composition_id,
                    "attempts": ctx.attempts,
                    "from_cache": ctx.from_cache,
                    "synthesized": synthesized_json,
                    "graph": serde_json::from_str::<serde_json::Value>(&graph_json).unwrap_or(json!(null)),
                    "output": exec_result.output,
                    "trace": exec_result.trace,
                }))
            );
        }
        Err(e) => {
            eprintln!("{}", acli_error(&format!("execution failed: {e}")));
            std::process::exit(3);
        }
    }
}
