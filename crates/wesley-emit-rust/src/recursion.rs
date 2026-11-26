//! Which references need a `Box` so that recursive types have a finite size.
//!
//! A struct field or union variant that holds another type by value makes the
//! holder at least as large as the held type. When the held type can reach
//! the holder again by value, the size is infinite (E0072). A `Vec` already
//! adds indirection, so a list reference never contributes to such a cycle.
//!
//! An edge `holder -> held` is boxed exactly when `held` can reach `holder`
//! through by-value edges, that is, when both ends lie in the same strongly
//! connected component (a self reference is the one-type case). The answer
//! depends only on the graph, never on the order in which it is walked, so
//! the output is deterministic.

use std::collections::{BTreeMap, BTreeSet};

use wesley_core::{TypeKind, TypeReference, WesleyIR};

/// The by-value edges of `ir` that lie on a cycle, as
/// `(holder type name, field or union member name)` in source names.
pub(crate) fn boxed_edges(ir: &WesleyIR) -> BTreeSet<(String, String)> {
    let defined = ir
        .types
        .iter()
        .map(|type_def| type_def.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut edges = Vec::<(&str, &str, &str)>::new();

    for type_def in &ir.types {
        match type_def.kind {
            TypeKind::Object | TypeKind::Interface | TypeKind::InputObject => {
                for field in &type_def.fields {
                    if !is_list(&field.r#type) && defined.contains(field.r#type.base.as_str()) {
                        edges.push((&type_def.name, &field.name, &field.r#type.base));
                    }
                }
            }
            TypeKind::Union => {
                for member in &type_def.union_members {
                    if defined.contains(member.as_str()) {
                        edges.push((&type_def.name, member, member));
                    }
                }
            }
            TypeKind::Scalar | TypeKind::Enum => {}
        }
    }

    let mut adjacency = BTreeMap::<&str, BTreeSet<&str>>::new();
    for (holder, _, held) in &edges {
        adjacency.entry(holder).or_default().insert(held);
    }

    edges
        .iter()
        .filter(|(holder, _, held)| reaches(&adjacency, held, holder))
        .map(|(holder, via, _)| ((*holder).to_string(), (*via).to_string()))
        .collect()
}

fn is_list(type_ref: &TypeReference) -> bool {
    type_ref.is_list || !type_ref.list_wrappers.is_empty()
}

/// Whether `to` is reachable from `from` (a type reaches itself).
fn reaches(adjacency: &BTreeMap<&str, BTreeSet<&str>>, from: &str, to: &str) -> bool {
    let mut seen = BTreeSet::new();
    let mut stack = vec![from];
    while let Some(current) = stack.pop() {
        if current == to {
            return true;
        }
        if seen.insert(current) {
            if let Some(next) = adjacency.get(current) {
                stack.extend(next.iter().copied());
            }
        }
    }
    false
}
