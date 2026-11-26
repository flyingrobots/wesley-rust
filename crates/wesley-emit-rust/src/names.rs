//! One Rust name for every source name, unique where Rust needs it to be.
//!
//! GraphQL names that differ can normalise to the same Rust identifier
//! (`fooBar` and `foo_bar` both become `foo_bar`; `_Thing` and `Thing` both
//! become `Thing`). The emitter returns source text and has no error channel,
//! so it disambiguates instead of refusing: within one scope, names are taken
//! in source order, a name keeps its normalised form when that form is free,
//! and a later name whose form is taken gets the first numeric suffix that is
//! free (see [`unique_names`]). Wire names are untouched: fields and variants
//! keep their source names in `#[serde(rename)]`.
//!
//! The scopes are: type names (every emitted type plus each operation's
//! request and response), field names per struct, variant names per enum or
//! union, and LE-binary codec function names. Models and codecs read the same
//! table, so they always agree.

use std::collections::{BTreeMap, BTreeSet};

use wesley_core::{SchemaOperation, TypeKind, WesleyIR};
use wesley_rust_syntax::unique_names;

use crate::{
    is_builtin_scalar, operation_scope_name, rust_field_name, rust_type_name, rust_variant_name,
    to_snake_case,
};

/// A type the emitter names: a type of the IR, or a type it generates for an
/// operation (keyed by operation scope and root field name).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum TypeKey {
    Ir(String),
    Request(&'static str, String),
    Response(&'static str, String),
}

/// The Rust names of one IR and operation list.
#[derive(Debug, Clone, Default)]
pub(crate) struct Names {
    types: BTreeMap<TypeKey, String>,
    codec_functions: BTreeMap<TypeKey, String>,
    /// Per Rust struct name: source field name -> Rust field name.
    fields: BTreeMap<String, BTreeMap<String, String>>,
    /// Per Rust enum name: source value or member -> Rust variant name.
    variants: BTreeMap<String, BTreeMap<String, String>>,
}

impl Names {
    pub(crate) fn new(ir: &WesleyIR, operations: &[SchemaOperation]) -> Self {
        let roots = operations
            .iter()
            .map(|operation| operation.root_type_name.as_str())
            .collect::<BTreeSet<_>>();
        let emitted = ir
            .types
            .iter()
            .filter(|type_def| {
                !roots.contains(type_def.name.as_str())
                    && !(type_def.kind == TypeKind::Scalar && is_builtin_scalar(&type_def.name))
            })
            .collect::<Vec<_>>();

        let mut keys = Vec::new();
        let mut sources = Vec::new();
        for type_def in &emitted {
            keys.push(TypeKey::Ir(type_def.name.clone()));
            sources.push(type_def.name.clone());
        }
        for operation in operations {
            let scope = operation_scope_name(operation.operation_type);
            let field = rust_type_name(&operation.field_name);
            keys.push(TypeKey::Request(scope, operation.field_name.clone()));
            sources.push(format!("{scope}{field}Request"));
            keys.push(TypeKey::Response(scope, operation.field_name.clone()));
            sources.push(format!("{scope}{field}Response"));
        }
        let rust = unique_names(&sources, rust_type_name);
        // Codec functions are named after the snake_case source name of an IR
        // type and the snake_case Rust name of a request.
        let codec_sources = keys
            .iter()
            .zip(&rust)
            .map(|(key, rust)| match key {
                TypeKey::Ir(source) => source.clone(),
                TypeKey::Request(..) | TypeKey::Response(..) => rust.clone(),
            })
            .collect::<Vec<_>>();
        let codec = unique_names(&codec_sources, to_snake_case);

        let mut names = Self {
            types: keys.iter().cloned().zip(rust).collect(),
            codec_functions: keys.into_iter().zip(codec).collect(),
            ..Self::default()
        };

        for type_def in &emitted {
            let rust = names.type_name(&type_def.name);
            match type_def.kind {
                TypeKind::Object | TypeKind::Interface | TypeKind::InputObject => {
                    let sources = type_def
                        .fields
                        .iter()
                        .map(|field| field.name.clone())
                        .collect::<Vec<_>>();
                    names.insert_fields(rust, &sources);
                }
                TypeKind::Enum => names.insert_variants(rust, &type_def.enum_values),
                TypeKind::Union => names.insert_variants(rust, &type_def.union_members),
                TypeKind::Scalar => {}
            }
        }
        for operation in operations {
            let rust = names.request_name(
                operation_scope_name(operation.operation_type),
                &operation.field_name,
            );
            let sources = operation
                .arguments
                .iter()
                .map(|argument| argument.name.clone())
                .collect::<Vec<_>>();
            names.insert_fields(rust, &sources);
        }

        names
    }

    fn insert_fields(&mut self, holder: String, sources: &[String]) {
        let rust = unique_names(sources, rust_field_name);
        self.fields
            .insert(holder, sources.iter().cloned().zip(rust).collect());
    }

    fn insert_variants(&mut self, holder: String, sources: &[String]) {
        let rust = unique_names(sources, rust_variant_name);
        self.variants
            .insert(holder, sources.iter().cloned().zip(rust).collect());
    }

    /// The Rust name of the IR type `source`; a name the table does not hold
    /// (a type the IR does not define) is normalised on its own.
    pub(crate) fn type_name(&self, source: &str) -> String {
        self.types
            .get(&TypeKey::Ir(source.to_string()))
            .cloned()
            .unwrap_or_else(|| rust_type_name(source))
    }

    /// The Rust name of the request struct for `scope`'s root field `field`.
    pub(crate) fn request_name(&self, scope: &'static str, field: &str) -> String {
        self.types
            .get(&TypeKey::Request(scope, field.to_string()))
            .cloned()
            .unwrap_or_else(|| rust_type_name(&format!("{scope}{}Request", rust_type_name(field))))
    }

    /// The Rust name of the response alias for `scope`'s root field `field`.
    pub(crate) fn response_name(&self, scope: &'static str, field: &str) -> String {
        self.types
            .get(&TypeKey::Response(scope, field.to_string()))
            .cloned()
            .unwrap_or_else(|| rust_type_name(&format!("{scope}{}Response", rust_type_name(field))))
    }

    /// The snake_case stem of the codec functions for the IR type `source`.
    pub(crate) fn codec_function(&self, source: &str) -> String {
        self.codec_functions
            .get(&TypeKey::Ir(source.to_string()))
            .cloned()
            .unwrap_or_else(|| to_snake_case(source))
    }

    /// The snake_case stem of the codec functions for a request struct.
    pub(crate) fn request_codec_function(&self, scope: &'static str, field: &str) -> String {
        self.codec_functions
            .get(&TypeKey::Request(scope, field.to_string()))
            .cloned()
            .unwrap_or_else(|| to_snake_case(&self.request_name(scope, field)))
    }

    /// The Rust name of field `source` in the struct named `holder` (Rust).
    pub(crate) fn field_name(&self, holder: &str, source: &str) -> String {
        self.fields
            .get(holder)
            .and_then(|fields| fields.get(source))
            .cloned()
            .unwrap_or_else(|| rust_field_name(source))
    }

    /// The Rust name of variant `source` in the enum named `holder` (Rust).
    pub(crate) fn variant_name(&self, holder: &str, source: &str) -> String {
        self.variants
            .get(holder)
            .and_then(|variants| variants.get(source))
            .cloned()
            .unwrap_or_else(|| rust_variant_name(source))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(names: &[&str]) -> Vec<String> {
        names.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn unique_names_keeps_free_forms_and_suffixes_later_duplicates() {
        assert_eq!(
            unique_names(
                &strings(&["fooBar", "foo_bar", "foo_bar_2", "type", "Type"]),
                rust_field_name
            ),
            strings(&["foo_bar", "foo_bar_3", "foo_bar_2", "r#type", "type_2"])
        );
        assert_eq!(
            unique_names(&strings(&["Thing", "_Thing", "Thing_"]), rust_type_name),
            strings(&["Thing", "Thing2", "Thing3"])
        );
        assert_eq!(
            unique_names(&strings(&["fooBar", "FOO_BAR"]), rust_variant_name),
            strings(&["FooBar", "FooBar2"])
        );
    }

    #[test]
    fn unique_names_leaves_distinct_names_alone() {
        let names = strings(&["id", "displayName", "createdAt"]);
        assert_eq!(
            unique_names(&names, rust_field_name),
            strings(&["id", "display_name", "created_at"])
        );
    }
}
