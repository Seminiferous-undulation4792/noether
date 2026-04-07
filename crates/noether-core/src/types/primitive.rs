use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The core type in Noether's structural type system.
///
/// Types are structural, not nominal: two types are compatible if their
/// structure matches, regardless of how they were named or created.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum NType {
    // Primitives (ordered by discriminant for stable Ord)
    Any,
    Bool,
    Bytes,
    List(Box<NType>),
    Map { key: Box<NType>, value: Box<NType> },
    Null,
    Number,
    Record(BTreeMap<String, NType>),
    Stream(Box<NType>),
    Text,
    Union(Vec<NType>),
    /// A virtual DOM node — the output type for UI component stages.
    ///
    /// VNode is opaque in the type system: it does not expose its internal
    /// tag/props/children structure as sub-types. The JS reactive runtime owns
    /// VNode semantics; the type checker only needs to know a VNode is a VNode.
    VNode,
}

impl NType {
    /// Create a normalized union type.
    ///
    /// Flattens nested unions, deduplicates, and sorts variants.
    /// Returns the inner type if only one variant remains.
    pub fn union(variants: Vec<NType>) -> NType {
        let mut flat = Vec::new();
        for v in variants {
            match v {
                NType::Union(inner) => flat.extend(inner),
                other => flat.push(other),
            }
        }
        flat.sort();
        flat.dedup();
        match flat.len() {
            0 => NType::Null,
            1 => flat.into_iter().next().unwrap(),
            _ => NType::Union(flat),
        }
    }

    /// Convenience for optional types: `T | Null`.
    pub fn optional(inner: NType) -> NType {
        NType::union(vec![inner, NType::Null])
    }

    /// Create a Record from field pairs.
    pub fn record(fields: impl IntoIterator<Item = (impl Into<String>, NType)>) -> NType {
        NType::Record(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_flattens_nested() {
        let inner = NType::Union(vec![NType::Text, NType::Number]);
        let outer = NType::union(vec![inner, NType::Bool]);
        assert_eq!(
            outer,
            NType::Union(vec![NType::Bool, NType::Number, NType::Text])
        );
    }

    #[test]
    fn union_deduplicates() {
        let u = NType::union(vec![NType::Text, NType::Text, NType::Number]);
        assert_eq!(u, NType::Union(vec![NType::Number, NType::Text]));
    }

    #[test]
    fn union_single_variant_unwraps() {
        let u = NType::union(vec![NType::Text]);
        assert_eq!(u, NType::Text);
    }

    #[test]
    fn union_empty_becomes_null() {
        let u = NType::union(vec![]);
        assert_eq!(u, NType::Null);
    }

    #[test]
    fn union_is_sorted() {
        let u = NType::union(vec![NType::Text, NType::Bool, NType::Number]);
        assert_eq!(
            u,
            NType::Union(vec![NType::Bool, NType::Number, NType::Text])
        );
    }

    #[test]
    fn optional_creates_union_with_null() {
        let opt = NType::optional(NType::Text);
        assert_eq!(opt, NType::Union(vec![NType::Null, NType::Text]));
    }

    #[test]
    fn serde_round_trip() {
        let types = vec![
            NType::Text,
            NType::Number,
            NType::List(Box::new(NType::Text)),
            NType::Map {
                key: Box::new(NType::Text),
                value: Box::new(NType::Number),
            },
            NType::record([("name", NType::Text), ("age", NType::Number)]),
            NType::union(vec![NType::Text, NType::Null]),
            NType::Stream(Box::new(NType::Bool)),
            NType::Any,
            NType::VNode,
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let deserialized: NType = serde_json::from_str(&json).unwrap();
            assert_eq!(t, deserialized);
        }
    }

    #[test]
    fn vnode_ord_after_union() {
        // VNode sorts after Union alphabetically, which keeps Ord stable.
        assert!(NType::VNode > NType::Union(vec![NType::Text]));
    }
}
