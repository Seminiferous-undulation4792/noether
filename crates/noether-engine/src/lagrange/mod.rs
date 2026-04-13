mod ast;

pub use ast::{collect_stage_ids, CompositionGraph, CompositionNode};

use noether_core::stage::StageId;
use noether_store::StageStore;
use sha2::{Digest, Sha256};

/// Parse a Lagrange JSON string into a CompositionGraph.
pub fn parse_graph(json: &str) -> Result<CompositionGraph, serde_json::Error> {
    serde_json::from_str(json)
}

/// Errors raised by `resolve_stage_prefixes` when an ID in the graph cannot
/// be uniquely resolved against the store.
#[derive(Debug, Clone)]
pub enum PrefixResolutionError {
    /// The prefix did not match any stage in the store.
    NotFound { prefix: String },
    /// The prefix matched multiple stages — author must use a longer prefix.
    Ambiguous {
        prefix: String,
        matches: Vec<String>,
    },
}

impl std::fmt::Display for PrefixResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { prefix } => {
                write!(f, "no stage in store matches prefix '{prefix}'")
            }
            Self::Ambiguous { prefix, matches } => {
                write!(
                    f,
                    "stage prefix '{prefix}' is ambiguous; matches {} stages — \
                     use a longer prefix. First few: {}",
                    matches.len(),
                    matches
                        .iter()
                        .take(3)
                        .map(|s| &s[..16.min(s.len())])
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

impl std::error::Error for PrefixResolutionError {}

/// Walk a composition graph and replace any stage IDs that are unique
/// prefixes of a real stage in the store with their full 64-character IDs.
///
/// Exact matches are passed through unchanged. Hand-authored graphs can
/// therefore use 8-character prefixes (the same form `noether stage list`
/// prints) without manually looking up the full hash.
pub fn resolve_stage_prefixes(
    node: &mut CompositionNode,
    store: &(impl StageStore + ?Sized),
) -> Result<(), PrefixResolutionError> {
    // Snapshot the IDs once — repeated walks would otherwise pay for it per node.
    let ids: Vec<String> = store.list(None).iter().map(|s| s.id.0.clone()).collect();
    resolve_in_node(node, &ids)
}

fn resolve_in_node(
    node: &mut CompositionNode,
    all_ids: &[String],
) -> Result<(), PrefixResolutionError> {
    match node {
        CompositionNode::Stage { id, .. } => {
            // Exact match: nothing to do.
            if all_ids.iter().any(|i| i == &id.0) {
                return Ok(());
            }
            // Otherwise, look for prefix matches.
            let matches: Vec<&String> = all_ids.iter().filter(|i| i.starts_with(&id.0)).collect();
            match matches.len() {
                0 => Err(PrefixResolutionError::NotFound {
                    prefix: id.0.clone(),
                }),
                1 => {
                    *id = StageId(matches[0].clone());
                    Ok(())
                }
                _ => Err(PrefixResolutionError::Ambiguous {
                    prefix: id.0.clone(),
                    matches: matches.into_iter().cloned().collect(),
                }),
            }
        }
        CompositionNode::RemoteStage { .. } | CompositionNode::Const { .. } => Ok(()),
        CompositionNode::Sequential { stages } => {
            for s in stages {
                resolve_in_node(s, all_ids)?;
            }
            Ok(())
        }
        CompositionNode::Parallel { branches } => {
            for b in branches.values_mut() {
                resolve_in_node(b, all_ids)?;
            }
            Ok(())
        }
        CompositionNode::Branch {
            predicate,
            if_true,
            if_false,
        } => {
            resolve_in_node(predicate, all_ids)?;
            resolve_in_node(if_true, all_ids)?;
            resolve_in_node(if_false, all_ids)
        }
        CompositionNode::Fanout { source, targets } => {
            resolve_in_node(source, all_ids)?;
            for t in targets {
                resolve_in_node(t, all_ids)?;
            }
            Ok(())
        }
        CompositionNode::Merge { sources, target } => {
            for s in sources {
                resolve_in_node(s, all_ids)?;
            }
            resolve_in_node(target, all_ids)
        }
        CompositionNode::Retry { stage, .. } => resolve_in_node(stage, all_ids),
        CompositionNode::Let { bindings, body } => {
            for b in bindings.values_mut() {
                resolve_in_node(b, all_ids)?;
            }
            resolve_in_node(body, all_ids)
        }
    }
}

/// Serialize a CompositionGraph to pretty-printed JSON.
pub fn serialize_graph(graph: &CompositionGraph) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(graph)
}

/// Compute a deterministic composition ID (SHA-256 of canonical JSON).
pub fn compute_composition_id(graph: &CompositionGraph) -> Result<String, serde_json::Error> {
    let bytes = serde_json::to_vec(graph)?;
    let hash = Sha256::digest(&bytes);
    Ok(hex::encode(hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lagrange::ast::CompositionNode;
    use noether_core::stage::StageId;

    #[test]
    fn parse_and_serialize_round_trip() {
        let graph = CompositionGraph::new(
            "test",
            CompositionNode::Stage {
                id: StageId("abc".into()),
                config: None,
            },
        );
        let json = serialize_graph(&graph).unwrap();
        let parsed = parse_graph(&json).unwrap();
        assert_eq!(graph, parsed);
    }

    #[test]
    fn composition_id_is_deterministic() {
        let graph = CompositionGraph::new(
            "test",
            CompositionNode::Stage {
                id: StageId("abc".into()),
                config: None,
            },
        );
        let id1 = compute_composition_id(&graph).unwrap();
        let id2 = compute_composition_id(&graph).unwrap();
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 64);
    }
}
