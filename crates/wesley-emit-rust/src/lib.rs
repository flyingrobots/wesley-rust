#![deny(warnings)]
#![deny(missing_docs)]

//! AST-based Rust model emitter for Wesley L1 IR.

use std::collections::BTreeSet;
use std::fmt::Write;

use names::Names;
use wesley_core::{
    LawEntryBodyV1, LawIrV1, OperationArgument, OperationType, ScalarRepresentationV1,
    SchemaOperation, TypeDefinition, TypeKind, TypeReference, WesleyIR,
};
use wesley_rust_syntax::naming::{
    field_name as rust_field_name, snake_case as to_snake_case, type_name as rust_type_name,
    variant_name as rust_variant_name,
};

mod le_binary;
mod names;
mod recursion;
pub use le_binary::{
    emit_le_binary_rust, DEFAULT_CODEC_IMPORT as LE_BINARY_RUST_DEFAULT_CODEC_IMPORT,
    LE_BINARY_GENERATOR_NAME as LE_BINARY_RUST_GENERATOR_NAME,
};

/// Stable generator identifier recorded in native emit metadata.
pub const GENERATOR_NAME: &str = "wesley-emit-rust";

/// Version of the Rust emitter crate recorded in native emit metadata.
pub const GENERATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Emits Rust model declarations for a Wesley L1 IR document.
pub fn emit_rust(ir: &WesleyIR) -> String {
    let file = RustFile::from_ir(ir);

    print_file(&file)
}

/// Emits Rust model and operation binding declarations for a Wesley L1 IR document.
pub fn emit_rust_with_operations(ir: &WesleyIR, operations: &[SchemaOperation]) -> String {
    let file = RustFile::from_ir_and_operations(ir, operations);

    print_file(&file)
}

/// Emits Rust declarations with schema and law hash traceability constants.
pub fn emit_rust_with_operations_and_hashes(
    ir: &WesleyIR,
    operations: &[SchemaOperation],
    schema_hash: &str,
    law_hash: &str,
) -> String {
    let mut file = RustFile::from_ir_and_operations(ir, operations);
    file.provenance = Some(RustProvenanceConstants {
        schema_hash: schema_hash.to_string(),
        law_hash: law_hash.to_string(),
    });

    print_file(&file)
}

/// Emits Rust declarations with provenance constants and law-backed helper validators.
pub fn emit_rust_with_operations_and_law(
    ir: &WesleyIR,
    operations: &[SchemaOperation],
    schema_hash: &str,
    law_hash: &str,
    law_ir: &LawIrV1,
) -> String {
    let mut file = RustFile::from_ir_and_operations(ir, operations);
    file.provenance = Some(RustProvenanceConstants {
        schema_hash: schema_hash.to_string(),
        law_hash: law_hash.to_string(),
    });
    file.law_items = rust_law_items_from_ir(ir, law_ir, &Names::new(ir, operations));

    print_file(&file)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustFile {
    provenance: Option<RustProvenanceConstants>,
    items: Vec<RustItem>,
    law_items: Vec<RustLawItem>,
}

impl RustFile {
    fn from_ir(ir: &WesleyIR) -> Self {
        Self::from_ir_and_operations(ir, &[])
    }

    fn from_ir_and_operations(ir: &WesleyIR, operations: &[SchemaOperation]) -> Self {
        let boxed = recursion::boxed_edges(ir);
        let names = Names::new(ir, operations);
        let mut items = Vec::new();
        let root_type_names = operations
            .iter()
            .map(|operation| operation.root_type_name.as_str())
            .collect::<BTreeSet<_>>();

        for type_def in &ir.types {
            if root_type_names.contains(type_def.name.as_str()) {
                continue;
            }

            match type_def.kind {
                TypeKind::Scalar if is_builtin_scalar(&type_def.name) => {}
                TypeKind::Scalar => items.push(RustItem::TypeAlias(RustTypeAlias {
                    name: names.type_name(&type_def.name),
                    target: RustType::String,
                })),
                TypeKind::Object | TypeKind::Interface | TypeKind::InputObject => {
                    items.push(RustItem::Struct(struct_from_type(type_def, &boxed, &names)));
                }
                TypeKind::Enum => items.push(RustItem::Enum(enum_from_type(type_def, &names))),
                TypeKind::Union => {
                    items.push(RustItem::Enum(union_enum_from_type(
                        type_def, &boxed, &names,
                    )));
                }
            }
        }

        items.extend(operations.iter().map(|operation| {
            RustItem::Operation(operation_binding_from_schema(operation, &names))
        }));

        Self {
            provenance: None,
            items,
            law_items: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustProvenanceConstants {
    schema_hash: String,
    law_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RustItem {
    TypeAlias(RustTypeAlias),
    Struct(RustStruct),
    Enum(RustEnum),
    Operation(RustOperationBinding),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RustLawItem {
    ScalarValidator(RustScalarValidator),
    VariantValidator(RustVariantValidator),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustTypeAlias {
    name: String,
    target: RustType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustStruct {
    name: String,
    fields: Vec<RustField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustField {
    source_name: String,
    rust_name: String,
    ty: RustType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustEnum {
    name: String,
    derive_eq: bool,
    variants: Vec<RustVariant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustOperationBinding {
    operation_type: &'static str,
    field_name: String,
    request: RustStruct,
    response_alias: RustTypeAlias,
    directives_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustVariant {
    source_name: String,
    rust_name: String,
    payload: Option<RustType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustScalarValidator {
    function_name: String,
    scalar_name: String,
    min_inclusive: Option<u64>,
    max_inclusive: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustVariantValidator {
    function_name: String,
    input_type: String,
    discriminator_field: String,
    enum_type: String,
    cases: Vec<RustVariantValidatorCase>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustVariantValidatorCase {
    enum_variant: String,
    case_value: String,
    requires: Vec<String>,
    forbids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RustType {
    String,
    I32,
    F64,
    Bool,
    Named(String),
    Vec(Box<RustType>),
    Option(Box<RustType>),
    Box(Box<RustType>),
}

fn rust_law_items_from_ir(ir: &WesleyIR, law_ir: &LawIrV1, names: &Names) -> Vec<RustLawItem> {
    let mut items = law_items_in_law_order(ir, law_ir, names);

    // Validator functions share one scope: names are made unique in law
    // order, as every other generated name is (see `names`).
    let function_names = items
        .iter()
        .map(|item| match item {
            RustLawItem::ScalarValidator(validator) => validator.function_name.clone(),
            RustLawItem::VariantValidator(validator) => validator.function_name.clone(),
        })
        .collect::<Vec<_>>();
    for (item, unique) in items.iter_mut().zip(wesley_rust_syntax::unique_names(
        &function_names,
        to_snake_case,
    )) {
        match item {
            RustLawItem::ScalarValidator(validator) => validator.function_name = unique,
            RustLawItem::VariantValidator(validator) => validator.function_name = unique,
        }
    }

    items
}

fn law_items_in_law_order(ir: &WesleyIR, law_ir: &LawIrV1, names: &Names) -> Vec<RustLawItem> {
    law_ir
        .entries
        .iter()
        .filter_map(|entry| match &entry.body {
            LawEntryBodyV1::ScalarSemantics(body) => {
                scalar_validator_from_law(&entry.subject, body, names)
            }
            LawEntryBodyV1::VariantLaw(body) => {
                variant_validator_from_law(ir, &entry.subject, body, names)
            }
            LawEntryBodyV1::FootprintLaw(_)
            | LawEntryBodyV1::ChannelLaw(_)
            | LawEntryBodyV1::InvariantLaw(_) => None,
        })
        .collect()
}

fn scalar_validator_from_law(
    subject: &str,
    body: &wesley_core::ScalarSemanticsLawV1,
    names: &Names,
) -> Option<RustLawItem> {
    if body.representation != ScalarRepresentationV1::Integer {
        return None;
    }
    let scalar_name = subject.strip_prefix("scalar:")?;

    Some(RustLawItem::ScalarValidator(RustScalarValidator {
        // A stem inside a longer identifier: plain snake_case, never raw.
        function_name: format!("validate_{}", to_snake_case(scalar_name)),
        scalar_name: names.type_name(scalar_name),
        min_inclusive: body.min_inclusive,
        max_inclusive: body.max_inclusive,
    }))
}

fn variant_validator_from_law(
    ir: &WesleyIR,
    subject: &str,
    body: &wesley_core::VariantLawV1,
    names: &Names,
) -> Option<RustLawItem> {
    let input_name = subject.strip_prefix("input:")?;
    let input = ir.types.iter().find(|definition| {
        definition.kind == TypeKind::InputObject && definition.name == input_name
    })?;
    let discriminator = input
        .fields
        .iter()
        .find(|field| field.name == body.discriminator.field)?;
    let input_type = names.type_name(input_name);
    let enum_type = names.type_name(&body.discriminator.r#enum);
    let cases = body
        .cases
        .iter()
        .map(|case| RustVariantValidatorCase {
            enum_variant: names.variant_name(&enum_type, &case.value),
            case_value: case.value.clone(),
            requires: dynamic_variant_fields(input, &input_type, &case.requires, names),
            forbids: dynamic_variant_fields(input, &input_type, &case.forbids, names),
        })
        .collect();

    Some(RustLawItem::VariantValidator(RustVariantValidator {
        function_name: format!("validate_{}_variant", to_snake_case(input_name)),
        discriminator_field: names.field_name(&input_type, &discriminator.name),
        input_type,
        enum_type,
        cases,
    }))
}

fn dynamic_variant_fields(
    input: &TypeDefinition,
    input_type: &str,
    fields: &[String],
    names: &Names,
) -> Vec<String> {
    fields
        .iter()
        .filter_map(|field_name| {
            let field = input
                .fields
                .iter()
                .find(|field| field.name == *field_name)?;
            if matches!(
                rust_type_from_reference(&field.r#type, names),
                RustType::Option(_)
            ) {
                Some(names.field_name(input_type, field_name))
            } else {
                None
            }
        })
        .collect()
}

fn struct_from_type(
    type_def: &TypeDefinition,
    boxed: &BTreeSet<(String, String)>,
    names: &Names,
) -> RustStruct {
    let name = names.type_name(&type_def.name);
    RustStruct {
        fields: type_def
            .fields
            .iter()
            .map(|field| {
                let is_cycle_edge = boxed.contains(&(type_def.name.clone(), field.name.clone()));
                let ty = rust_type_from_reference(&field.r#type, names);
                RustField {
                    source_name: field.name.clone(),
                    rust_name: names.field_name(&name, &field.name),
                    ty: if is_cycle_edge { box_named(ty) } else { ty },
                }
            })
            .collect(),
        name,
    }
}

/// Puts the named type inside `ty` behind a `Box`: `Node` becomes `Box<Node>`
/// and `Option<Node>` becomes `Option<Box<Node>>`. Only by-value references
/// are cycle edges (see [`recursion`]), so a `Vec` never reaches here.
fn box_named(ty: RustType) -> RustType {
    match ty {
        RustType::Named(_) => RustType::Box(Box::new(ty)),
        RustType::Option(inner) => RustType::Option(Box::new(box_named(*inner))),
        other => other,
    }
}

fn enum_from_type(type_def: &TypeDefinition, names: &Names) -> RustEnum {
    let name = names.type_name(&type_def.name);
    RustEnum {
        derive_eq: true,
        variants: type_def
            .enum_values
            .iter()
            .map(|value| RustVariant {
                source_name: value.clone(),
                rust_name: names.variant_name(&name, value),
                payload: None,
            })
            .collect(),
        name,
    }
}

fn union_enum_from_type(
    type_def: &TypeDefinition,
    boxed: &BTreeSet<(String, String)>,
    names: &Names,
) -> RustEnum {
    let name = names.type_name(&type_def.name);
    RustEnum {
        derive_eq: false,
        variants: type_def
            .union_members
            .iter()
            .map(|member| {
                let payload = RustType::Named(names.type_name(member));
                let is_cycle_edge = boxed.contains(&(type_def.name.clone(), member.clone()));
                RustVariant {
                    source_name: member.clone(),
                    rust_name: names.variant_name(&name, member),
                    payload: Some(if is_cycle_edge {
                        box_named(payload)
                    } else {
                        payload
                    }),
                }
            })
            .collect(),
        name,
    }
}

fn operation_binding_from_schema(
    operation: &SchemaOperation,
    names: &Names,
) -> RustOperationBinding {
    let scope = operation_scope_name(operation.operation_type);
    let request_type_name = names.request_name(scope, &operation.field_name);
    let response_type_name = names.response_name(scope, &operation.field_name);

    RustOperationBinding {
        operation_type: operation_type_literal(operation.operation_type),
        field_name: operation.field_name.clone(),
        request: RustStruct {
            fields: operation
                .arguments
                .iter()
                .map(|argument| field_from_operation_argument(argument, &request_type_name, names))
                .collect(),
            name: request_type_name,
        },
        response_alias: RustTypeAlias {
            name: response_type_name,
            target: rust_type_from_reference(&operation.result_type, names),
        },
        directives_json: serde_json::to_string(&operation.directives)
            .expect("schema operation directives should serialize"),
    }
}

fn field_from_operation_argument(
    argument: &OperationArgument,
    request_type_name: &str,
    names: &Names,
) -> RustField {
    let mut ty = rust_type_from_reference(&argument.r#type, names);
    if argument.default_value.is_some() && !matches!(ty, RustType::Option(_)) {
        ty = RustType::Option(Box::new(ty));
    }

    RustField {
        source_name: argument.name.clone(),
        rust_name: names.field_name(request_type_name, &argument.name),
        ty,
    }
}

fn rust_type_from_reference(type_ref: &TypeReference, names: &Names) -> RustType {
    let base = match type_ref.base.as_str() {
        "ID" | "String" => RustType::String,
        "Int" => RustType::I32,
        "Float" => RustType::F64,
        "Boolean" => RustType::Bool,
        name => RustType::Named(names.type_name(name)),
    };

    if !type_ref.list_wrappers.is_empty() {
        let mut ty = if type_ref.leaf_nullable.unwrap_or(true) {
            RustType::Option(Box::new(base))
        } else {
            base
        };

        for wrapper in type_ref.list_wrappers.iter().rev() {
            ty = RustType::Vec(Box::new(ty));
            if wrapper.nullable {
                ty = RustType::Option(Box::new(ty));
            }
        }

        return ty;
    }

    let mut ty = if type_ref.is_list {
        let item = match type_ref.list_item_nullable {
            Some(true) | None => RustType::Option(Box::new(base)),
            Some(false) => base,
        };
        RustType::Vec(Box::new(item))
    } else {
        base
    };

    if type_ref.nullable {
        ty = RustType::Option(Box::new(ty));
    }

    ty
}

fn print_file(file: &RustFile) -> String {
    let mut out = String::from("// @generated by Wesley. Do not edit.\n");

    if let Some(provenance) = &file.provenance {
        out.push('\n');
        write!(out, "pub const WESLEY_SCHEMA_HASH: &'static str = ")
            .expect("writing to string should not fail");
        print_rust_string_literal(&mut out, &provenance.schema_hash);
        out.push_str(";\n");
        write!(out, "pub const WESLAW_HASH: &'static str = ")
            .expect("writing to string should not fail");
        print_rust_string_literal(&mut out, &provenance.law_hash);
        out.push_str(";\n");
    }

    for item in &file.items {
        out.push('\n');
        print_item(&mut out, item);
    }

    for item in &file.law_items {
        out.push('\n');
        print_law_item(&mut out, item);
    }

    out
}

fn print_item(out: &mut String, item: &RustItem) {
    match item {
        RustItem::TypeAlias(alias) => print_type_alias(out, alias),
        RustItem::Struct(struct_item) => print_struct(out, struct_item),
        RustItem::Enum(enum_item) => print_enum(out, enum_item),
        RustItem::Operation(operation) => print_operation_binding(out, operation),
    }
}

fn print_law_item(out: &mut String, item: &RustLawItem) {
    match item {
        RustLawItem::ScalarValidator(validator) => print_scalar_validator(out, validator),
        RustLawItem::VariantValidator(validator) => print_variant_validator(out, validator),
    }
}

fn print_scalar_validator(out: &mut String, validator: &RustScalarValidator) {
    writeln!(
        out,
        "pub fn {}(value: u64) -> Result<u64, &'static str> {{",
        validator.function_name
    )
    .expect("writing to string should not fail");
    if let Some(min) = validator.min_inclusive {
        writeln!(
            out,
            "    if value < {min} {{ return Err(\"{} below minInclusive {min}\"); }}",
            validator.scalar_name
        )
        .expect("writing to string should not fail");
    }
    if let Some(max) = validator.max_inclusive {
        writeln!(
            out,
            "    if value > {max} {{ return Err(\"{} above maxInclusive {max}\"); }}",
            validator.scalar_name
        )
        .expect("writing to string should not fail");
    }
    out.push_str("    Ok(value)\n");
    out.push_str("}\n");
}

fn print_variant_validator(out: &mut String, validator: &RustVariantValidator) {
    writeln!(
        out,
        "pub fn {}(value: &{}) -> Result<(), &'static str> {{",
        validator.function_name, validator.input_type
    )
    .expect("writing to string should not fail");
    writeln!(
        out,
        "    match value.{}.clone() {{",
        validator.discriminator_field
    )
    .expect("writing to string should not fail");
    for case in &validator.cases {
        writeln!(
            out,
            "        {}::{} => {{",
            validator.enum_type, case.enum_variant
        )
        .expect("writing to string should not fail");
        for field in &case.requires {
            write!(out, "            if value.{field}.is_none() {{ return Err(")
                .expect("writing to string should not fail");
            print_rust_string_literal(out, &format!("{} requires {field}", case.case_value));
            out.push_str("); }\n");
        }
        for field in &case.forbids {
            write!(out, "            if value.{field}.is_some() {{ return Err(")
                .expect("writing to string should not fail");
            print_rust_string_literal(out, &format!("{} forbids {field}", case.case_value));
            out.push_str("); }\n");
        }
        out.push_str("        }\n");
    }
    out.push_str("    }\n");
    out.push_str("    Ok(())\n");
    out.push_str("}\n");
}

fn print_type_alias(out: &mut String, alias: &RustTypeAlias) {
    write!(out, "pub type {} = ", alias.name).expect("writing to string should not fail");
    print_type(out, &alias.target);
    out.push_str(";\n");
}

fn print_struct(out: &mut String, struct_item: &RustStruct) {
    out.push_str("#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]\n");
    writeln!(out, "pub struct {} {{", struct_item.name).expect("writing to string should not fail");

    for field in &struct_item.fields {
        if field.source_name != field.rust_name.trim_start_matches("r#") {
            writeln!(
                out,
                "    #[serde(rename = \"{}\")]",
                escape_attribute(&field.source_name)
            )
            .expect("writing to string should not fail");
        }
        write!(out, "    pub {}: ", field.rust_name).expect("writing to string should not fail");
        print_type(out, &field.ty);
        out.push_str(",\n");
    }

    out.push_str("}\n");
}

fn print_enum(out: &mut String, enum_item: &RustEnum) {
    if enum_item.derive_eq {
        out.push_str(
            "#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]\n",
        );
    } else {
        out.push_str("#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]\n");
    }
    writeln!(out, "pub enum {} {{", enum_item.name).expect("writing to string should not fail");

    for variant in &enum_item.variants {
        writeln!(
            out,
            "    #[serde(rename = \"{}\")]",
            escape_attribute(&variant.source_name)
        )
        .expect("writing to string should not fail");
        write!(out, "    {}", variant.rust_name).expect("writing to string should not fail");
        if let Some(payload) = &variant.payload {
            out.push('(');
            print_type(out, payload);
            out.push(')');
        }
        out.push_str(",\n");
    }

    out.push_str("}\n");
}

fn print_operation_binding(out: &mut String, operation: &RustOperationBinding) {
    print_struct(out, &operation.request);
    out.push('\n');
    print_type_alias(out, &operation.response_alias);
    out.push('\n');
    writeln!(out, "impl {} {{", operation.request.name).expect("writing to string should not fail");
    write!(out, "    pub const OPERATION_TYPE: &'static str = ")
        .expect("writing to string should not fail");
    print_rust_string_literal(out, operation.operation_type);
    out.push_str(";\n");
    write!(out, "    pub const FIELD_NAME: &'static str = ")
        .expect("writing to string should not fail");
    print_rust_string_literal(out, &operation.field_name);
    out.push_str(";\n");
    write!(out, "    pub const DIRECTIVES_JSON: &'static str = ")
        .expect("writing to string should not fail");
    print_rust_string_literal(out, &operation.directives_json);
    out.push_str(";\n");
    out.push_str("}\n");
}

fn print_type(out: &mut String, ty: &RustType) {
    match ty {
        RustType::String => out.push_str("String"),
        RustType::I32 => out.push_str("i32"),
        RustType::F64 => out.push_str("f64"),
        RustType::Bool => out.push_str("bool"),
        RustType::Named(name) => out.push_str(name),
        RustType::Vec(item) => {
            out.push_str("Vec<");
            print_type(out, item);
            out.push('>');
        }
        RustType::Option(item) => {
            out.push_str("Option<");
            print_type(out, item);
            out.push('>');
        }
        RustType::Box(item) => {
            out.push_str("Box<");
            print_type(out, item);
            out.push('>');
        }
    }
}

fn operation_type_literal(operation_type: OperationType) -> &'static str {
    match operation_type {
        OperationType::Query => "QUERY",
        OperationType::Mutation => "MUTATION",
        OperationType::Subscription => "SUBSCRIPTION",
    }
}

fn is_builtin_scalar(name: &str) -> bool {
    matches!(name, "ID" | "String" | "Int" | "Float" | "Boolean")
}

fn operation_scope_name(operation_type: OperationType) -> &'static str {
    match operation_type {
        OperationType::Query => "Query",
        OperationType::Mutation => "Mutation",
        OperationType::Subscription => "Subscription",
    }
}

fn escape_attribute(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn print_rust_string_literal(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if control.is_control() => {
                write!(out, "\\u{{{:x}}}", control as u32)
                    .expect("writing to string should not fail");
            }
            _ => out.push(ch),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::{
        fs,
        path::PathBuf,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };
    use wesley_core::{list_schema_operations_sdl, load_weslaw_yaml, lower_schema_sdl};

    #[test]
    fn emits_rust_models_from_l1_ir() {
        let ir = lower_schema_sdl(
            r#"
            scalar DateTime

            enum Role {
              ADMIN
              READ_ONLY
            }

            type User {
              id: ID!
              displayName: String
              createdAt: DateTime!
              tags: [String]
            }

            input UserFilter {
              role: Role
              limit: Int!
            }
            "#,
        )
        .expect("schema should lower");

        let actual = emit_rust(&ir);

        syn::parse_file(&actual).expect("generated Rust should parse");
        assert_eq!(
            actual,
            r#"// @generated by Wesley. Do not edit.

pub type DateTime = String;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Role {
    #[serde(rename = "ADMIN")]
    Admin,
    #[serde(rename = "READ_ONLY")]
    ReadOnly,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime,
    pub tags: Option<Vec<Option<String>>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UserFilter {
    pub role: Option<Role>,
    pub limit: i32,
}
"#
        );
    }

    #[test]
    fn emits_nested_graphql_lists_as_nested_rust_vectors() {
        let ir = lower_schema_sdl(
            r#"
            type Matrix {
              values: [[Int!]!]!
              maybeValues: [[String]]
            }
            "#,
        )
        .expect("schema should lower");

        let actual = emit_rust(&ir);

        syn::parse_file(&actual).expect("generated Rust should parse");
        assert!(actual.contains("pub values: Vec<Vec<i32>>,"));
        assert!(actual.contains(
            "#[serde(rename = \"maybeValues\")]\n    pub maybe_values: Option<Vec<Option<Vec<Option<String>>>>>,"
        ));
    }

    #[test]
    fn emits_single_list_nullability_shapes() {
        let ir = lower_schema_sdl(
            r#"
            type ListShape {
              requiredListNullableItems: [String]!
              nullableListRequiredItems: [String!]
            }
            "#,
        )
        .expect("schema should lower");

        let actual = emit_rust(&ir);

        syn::parse_file(&actual).expect("generated Rust should parse");
        assert!(actual.contains(
            "#[serde(rename = \"requiredListNullableItems\")]\n    pub required_list_nullable_items: Vec<Option<String>>,"
        ));
        assert!(actual.contains(
            "#[serde(rename = \"nullableListRequiredItems\")]\n    pub nullable_list_required_items: Option<Vec<String>>,"
        ));
    }

    #[test]
    fn emits_jedit_shaped_hot_text_fixture() {
        let ir = lower_schema_sdl(include_str!(
            "../../../test/fixtures/consumer-models/jedit-hot-text-core.graphql"
        ))
        .expect("jedit-shaped fixture should lower");

        let actual = emit_rust(&ir);

        syn::parse_file(&actual).expect("generated Rust should parse");
        assert!(actual.contains("pub struct BufferWorldline {"));
        assert!(actual.contains("pub enum AnchorKind {"));
        assert!(actual.contains("pub checkpoints: Vec<Checkpoint>,"));
        assert!(actual.contains("pub created_at_tick_id: Option<String>,"));
        assert!(actual.contains("pub create_initial_checkpoint: Option<bool>,"));
    }

    #[test]
    fn emits_jedit_operation_bindings() {
        let sdl = include_str!("../../../test/fixtures/consumer-models/jedit-rope.graphql");
        let ir = lower_schema_sdl(sdl).expect("jedit runtime fixture should lower");
        let operations =
            list_schema_operations_sdl(sdl).expect("jedit runtime operations should resolve");

        let actual = emit_rust_with_operations(&ir, &operations);

        syn::parse_file(&actual).expect("generated Rust should parse");
        assert!(!actual.contains("pub struct Mutation {"));
        assert!(actual.contains("pub struct MutationCreateBufferWorldlineRequest {"));
        assert!(actual.contains("pub input: CreateBufferWorldlineInput,"));
        assert!(actual.contains(
            "pub type MutationCreateBufferWorldlineResponse = CreateBufferWorldlineResult;"
        ));
        assert!(actual.contains("pub const OPERATION_TYPE: &'static str = \"MUTATION\";"));
        assert!(actual.contains("pub const FIELD_NAME: &'static str = \"createBufferWorldline\";"));
        assert!(actual.contains("pub const DIRECTIVES_JSON: &'static str = "));
        assert!(actual.contains("\\\"wes_footprint\\\""));
    }

    #[test]
    fn emits_law_backed_scalar_and_variant_validators() {
        let sdl = include_str!("../../../test/fixtures/weslaw/contract-bundle-shape.graphql");
        let law_ir = load_weslaw_yaml(include_str!(
            "../../../test/fixtures/weslaw/accepted/rust-validator-payoff.weslaw.yaml"
        ))
        .expect("law fixture should lower");
        let ir = lower_schema_sdl(sdl).expect("schema should lower");
        let operations = list_schema_operations_sdl(sdl).expect("operations should resolve");

        let actual = emit_rust_with_operations_and_law(
            &ir,
            &operations,
            "sha256:schema",
            "sha256:law",
            &law_ir,
        );

        syn::parse_file(&actual).expect("generated Rust should parse");
        assert!(actual.contains("pub fn validate_positive_int(value: u64)"));
        assert!(actual.contains("if value < 1"));
        assert!(actual.contains("if value > 4294967295"));
        assert!(actual
            .contains("pub fn validate_playback_mode_input_variant(value: &PlaybackModeInput)"));
        assert!(actual.contains("PlaybackModeKind::Seek => {"));
        assert!(actual.contains("if value.target.is_none()"));
        assert!(actual.contains("if value.then.is_none()"));
        assert!(actual.contains("PlaybackModeKind::Paused => {"));
        assert!(actual.contains("if value.target.is_some()"));
    }

    #[test]
    fn escapes_law_variant_case_values_in_validator_messages() {
        let actual = hostile_variant_validator_source();

        syn::parse_file(&actual).expect("generated Rust validator should parse");
        assert!(actual.contains(
            "return Err(\"CASE \\\"quote\\\" \\\\slash\\nline\\rreturn\\t tab requires target\");"
        ));
        assert!(actual.contains(
            "return Err(\"CASE \\\"quote\\\" \\\\slash\\nline\\rreturn\\t tab forbids then\");"
        ));
    }

    #[test]
    fn escaped_law_variant_validator_compiles() {
        let source = format!(
            r#"
#[derive(Clone)]
enum PlaybackModeKind {{
    Seek,
}}

struct PlaybackModeInput {{
    kind: PlaybackModeKind,
    target: Option<u8>,
    then: Option<u8>,
}}

{validator}
"#,
            validator = hostile_variant_validator_source()
        );

        compile_rust_source("escaped_law_variant_validator", &source);
    }

    #[test]
    fn validators_for_keyword_named_types_compile() {
        let sdl = r#"
            scalar Gen

            enum Kind {
              ON
              OFF
            }

            input Type {
              kind: Kind!
              target: Gen
            }
        "#;
        let law_ir = load_weslaw_yaml(
            r#"
apiVersion: weslaw/v1
schema:
  family: keyword-named-validators
  hash: sha256:0000000000000000000000000000000000000000000000000000000000000000
  source: inline.graphql
laws:
  - id: keyword.scalar.gen
    status: active
    kind: scalarSemantics
    subject: scalar:Gen
    semantics:
      representation: integer
      minInclusive: 1
    rationale: A scalar whose snake_case name is a keyword.
  - id: keyword.variant.type
    status: active
    kind: variantLaw
    subject: input:Type
    discriminator:
      field: kind
      enum: Kind
    cases:
      - value: ON
        requires: [target]
      - value: OFF
        forbids: [target]
"#,
        )
        .expect("law should load");
        let ir = lower_schema_sdl(sdl).expect("schema should lower");

        let actual = emit_rust_with_operations_and_law(&ir, &[], "schema", "law", &law_ir);

        assert!(
            actual.contains("pub fn validate_gen(value: u64)"),
            "{actual}"
        );
        assert!(
            actual.contains("pub fn validate_type_variant(value: &Type)"),
            "{actual}"
        );
        compile_generated_files("keyword_named_validators", &actual, "");
    }

    #[test]
    fn validators_whose_names_collide_are_disambiguated_in_law_order() {
        let sdl = r#"
            scalar URL
            scalar Url
            scalar fooBar
            scalar foo_bar
        "#;
        let law_ir = load_weslaw_yaml(
            r#"
apiVersion: weslaw/v1
schema:
  family: colliding-validators
  hash: sha256:0000000000000000000000000000000000000000000000000000000000000000
  source: inline.graphql
laws:
  - id: collide.url.upper
    status: active
    kind: scalarSemantics
    subject: scalar:URL
    semantics:
      representation: integer
      minInclusive: 1
    rationale: Normalises to url.
  - id: collide.url.title
    status: active
    kind: scalarSemantics
    subject: scalar:Url
    semantics:
      representation: integer
      minInclusive: 2
    rationale: Also normalises to url.
  - id: collide.foo.camel
    status: active
    kind: scalarSemantics
    subject: scalar:fooBar
    semantics:
      representation: integer
      maxInclusive: 3
    rationale: Normalises to foo_bar.
  - id: collide.foo.snake
    status: active
    kind: scalarSemantics
    subject: scalar:foo_bar
    semantics:
      representation: integer
      maxInclusive: 4
    rationale: Also normalises to foo_bar.
"#,
        )
        .expect("law should load");
        let ir = lower_schema_sdl(sdl).expect("schema should lower");

        let actual = emit_rust_with_operations_and_law(&ir, &[], "schema", "law", &law_ir);

        for name in [
            "validate_url(",
            "validate_url_2(",
            "validate_foo_bar(",
            "validate_foo_bar_2(",
        ] {
            assert!(actual.contains(&format!("pub fn {name}")), "{actual}");
        }
        compile_generated_files("colliding_validators", &actual, "");
    }

    #[test]
    fn law_backed_generated_rust_compiles_as_crate() {
        let sdl = include_str!("../../../test/fixtures/weslaw/contract-bundle-shape.graphql");
        let law_ir = load_weslaw_yaml(include_str!(
            "../../../test/fixtures/weslaw/accepted/rust-validator-payoff.weslaw.yaml"
        ))
        .expect("law fixture should lower");
        let ir = lower_schema_sdl(sdl).expect("fixture should lower");
        let operations = list_schema_operations_sdl(sdl).expect("operations should list");
        let actual =
            emit_rust_with_operations_and_law(&ir, &operations, "schema-hash", "law-hash", &law_ir);

        cargo_check_generated_rust_crate("law_backed_generated_rust", &actual);
    }

    #[test]
    fn variant_validator_printer_does_not_directly_interpolate_case_values() {
        let source = include_str!("lib.rs");
        for pattern in [
            concat!("Err(", "\\", "\"", "{} requires"),
            concat!("Err(", "\\", "\"", "{} forbids"),
        ] {
            assert!(
                !source.contains(pattern),
                "variant validator printer must route case values through the Rust string literal printer"
            );
        }
    }

    #[test]
    fn rust_string_literal_escapes_remaining_control_characters() {
        let mut actual = String::new();
        print_rust_string_literal(&mut actual, "nul:\0 esc:\u{1b} delete:\u{7f}");

        assert_eq!(actual, "\"nul:\\u{0} esc:\\u{1b} delete:\\u{7f}\"");
        assert!(
            !actual.trim_matches('"').chars().any(char::is_control),
            "generated string literal should not contain raw control characters"
        );
    }

    #[test]
    fn emits_stack_witness_0001_fixture_operation_bindings() {
        let sdl = include_str!(
            "../../../test/fixtures/consumer-models/stack-witness-0001-file-history.graphql"
        );
        let ir = lower_schema_sdl(sdl).expect("stack witness fixture should lower");
        let operations =
            list_schema_operations_sdl(sdl).expect("stack witness operations should resolve");

        let actual = emit_rust_with_operations(&ir, &operations);

        syn::parse_file(&actual).expect("generated Rust should parse");
        assert!(actual.contains("pub struct MutationCreateBufferRequest {"));
        assert!(actual.contains("pub input: CreateBufferInput,"));
        assert!(actual.contains("pub type MutationCreateBufferResponse = MutationReceipt;"));
        assert!(actual.contains("pub struct MutationReplaceRangeRequest {"));
        assert!(actual.contains("pub type MutationReplaceRangeResponse = MutationReceipt;"));
        assert!(actual.contains("pub struct QueryTextWindowRequest {"));
        assert!(actual.contains("pub type QueryTextWindowResponse = TextWindowReading;"));
        assert!(actual.contains("pub data_base64: String,"));

        let create_buffer = rust_operation_impl_block(&actual, "MutationCreateBufferRequest");
        assert!(create_buffer.contains("pub const FIELD_NAME: &'static str = \"createBuffer\";"));
        assert!(create_buffer.contains("\\\"artifactId\\\":\\\"fixture-file-history-v0\\\""));
        assert!(create_buffer.contains("\\\"opId\\\":\\\"0x53570001\\\""));
        assert!(create_buffer.contains("\\\"helperKind\\\":\\\"EINT\\\""));
        assert!(create_buffer.contains("\\\"targetCodec\\\":\\\"wesley-binary/v0\\\""));
        assert!(create_buffer.contains(
            "\\\"fixtureVarsBytes\\\":\\\"stack-witness-0001/createBuffer;name=demo.txt;artifact=fixture-file-history-v0\\\""
        ));

        let replace_range = rust_operation_impl_block(&actual, "MutationReplaceRangeRequest");
        assert!(replace_range.contains("pub const FIELD_NAME: &'static str = \"replaceRange\";"));
        assert!(replace_range.contains("\\\"artifactId\\\":\\\"fixture-file-history-v0\\\""));
        assert!(replace_range.contains("\\\"opId\\\":\\\"0x53570002\\\""));
        assert!(replace_range.contains("\\\"helperKind\\\":\\\"EINT\\\""));
        assert!(replace_range.contains("\\\"targetCodec\\\":\\\"wesley-binary/v0\\\""));
        assert!(replace_range.contains(
            "\\\"fixtureVarsBytes\\\":\\\"stack-witness-0001/replaceRange;bufferId=demo.txt;basis=B0;coord=utf8-bytes;start=0;end=0;text=hello;artifact=fixture-file-history-v0\\\""
        ));

        let text_window = rust_operation_impl_block(&actual, "QueryTextWindowRequest");
        assert!(text_window.contains("pub const FIELD_NAME: &'static str = \"textWindow\";"));
        assert!(text_window.contains("\\\"artifactId\\\":\\\"fixture-file-history-v0\\\""));
        assert!(text_window.contains("\\\"opId\\\":\\\"0x53571001\\\""));
        assert!(text_window.contains("\\\"helperKind\\\":\\\"QueryView\\\""));
        assert!(text_window.contains("\\\"targetCodec\\\":\\\"wesley-binary/v0\\\""));
        assert!(text_window.contains(
            "\\\"fixtureVarsBytes\\\":\\\"stack-witness-0001/textWindow;bufferId=demo.txt;basis=B1;coord=utf8-bytes;start=0;length=5;artifact=fixture-file-history-v0\\\""
        ));
        assert!(text_window.contains("\\\"payloadCodec\\\":\\\"QueryBytes\\\""));
        assert!(text_window.contains("\\\"envelope\\\":\\\"ReadingEnvelope\\\""));
    }

    #[test]
    fn emits_domain_empty_fixture_without_product_nouns() {
        let sdl = include_str!("../../../test/fixtures/rust-emitter/domain-empty.graphql");
        let ir = lower_schema_sdl(sdl).expect("domain-empty fixture should lower");
        let operations = list_schema_operations_sdl(sdl)
            .expect("domain-empty fixture operations should resolve");

        let actual = emit_rust_with_operations(&ir, &operations);

        syn::parse_file(&actual).expect("generated Rust should parse");
        assert!(actual.contains("pub struct ContractRecord {"));
        assert!(actual.contains("pub struct MutationPublishContractRequest {"));
        assert!(actual.contains("pub type QueryContractResponse = Option<ContractRecord>;"));
        for forbidden in ["Postgres", "Supabase", "Echo", "jedit", "Jedit"] {
            assert!(
                !actual.contains(forbidden),
                "generic Rust output leaked product noun `{forbidden}`"
            );
        }
    }

    #[test]
    fn operation_bindings_include_operation_scope_in_type_names() {
        let sdl = r#"
            type Query {
              status: Status!
            }

            type Mutation {
              status(input: StatusInput!): Status!
            }

            type Status {
              ok: Boolean!
            }

            input StatusInput {
              reason: String
            }
        "#;
        let ir = lower_schema_sdl(sdl).expect("schema should lower");
        let operations = list_schema_operations_sdl(sdl).expect("operations should resolve");

        let actual = emit_rust_with_operations(&ir, &operations);

        syn::parse_file(&actual).expect("generated Rust should parse");
        assert!(actual.contains("pub struct QueryStatusRequest {"));
        assert!(actual.contains("pub type QueryStatusResponse = Status;"));
        assert!(actual.contains("pub struct MutationStatusRequest {"));
        assert!(actual.contains("pub type MutationStatusResponse = Status;"));
    }

    #[test]
    fn sanitizes_reserved_field_names() {
        assert_eq!(rust_field_name("type"), "r#type");
        assert_eq!(rust_field_name("displayName"), "display_name");
    }

    #[test]
    fn names_come_from_the_shared_syntax_crate() {
        let ir = lower_schema_sdl(
            r#"
            type Example {
              self: String!
              userID: ID!
            }
            "#,
        )
        .expect("schema should lower");

        let actual = emit_rust(&ir);

        syn::parse_file(&actual).expect("generated Rust should parse");
        assert!(
            actual.contains("#[serde(rename = \"self\")]\n    pub self_: String,"),
            "{actual}"
        );
        assert!(
            actual.contains("#[serde(rename = \"userID\")]\n    pub user_id: String,"),
            "{actual}"
        );
        assert_eq!(rust_type_name("foo_bar"), "FooBar");
        assert_eq!(rust_variant_name("READ_ONLY"), "ReadOnly");
    }

    #[test]
    fn union_payload_enums_do_not_derive_eq() {
        let ir = lower_schema_sdl(
            r#"
            type User {
              id: ID!
            }

            union SearchResult = User
            "#,
        )
        .expect("schema should lower");

        let actual = emit_rust(&ir);

        syn::parse_file(&actual).expect("generated Rust should parse");
        assert!(actual.contains(
            "#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]\npub enum SearchResult"
        ));
    }

    #[test]
    fn float_fields_compile_with_their_le_binary_codecs() {
        compile_models_and_codecs(
            "float_codecs",
            r#"
            type Measurement {
              ratio: Float!
              maybeRatio: Float
              samples: [Float!]!
            }

            type Query {
              measurement(scale: Float!, offset: Float): Measurement!
            }
            "#,
        );
    }

    #[test]
    fn directly_recursive_types_compile_with_their_codecs() {
        compile_models_and_codecs(
            "direct_recursion",
            r#"
            type Node {
              next: Node
              parent: Node!
              children: [Node!]!
            }

            input Filter {
              not: Filter
              and: [Filter!]
            }

            type Query {
              node(filter: Filter!): Node
            }
            "#,
        );
    }

    #[test]
    fn mutually_recursive_types_compile_with_their_codecs() {
        compile_models_and_codecs(
            "mutual_recursion",
            r#"
            type Author {
              favourite: Book
              books: [Book!]!
            }

            type Book {
              author: Author!
              title: String!
            }

            type Query {
              book: Book
            }
            "#,
        );
    }

    #[test]
    fn recursion_through_a_union_compiles() {
        let ir = lower_schema_sdl(
            r#"
            type Leaf {
              value: Int!
            }

            type Branch {
              left: Tree!
              right: Tree
            }

            union Tree = Leaf | Branch
            "#,
        )
        .expect("schema should lower");

        compile_generated_files("union_recursion", &emit_rust(&ir), "");
    }

    #[test]
    fn boxes_only_the_edges_of_a_cycle_that_are_not_behind_a_vec() {
        let ir = lower_schema_sdl(
            r#"
            type Node {
              next: Node
              parent: Node!
              children: [Node!]!
              leaf: Leaf
            }

            type Leaf {
              value: Int!
            }
            "#,
        )
        .expect("schema should lower");

        let actual = emit_rust(&ir);

        assert!(actual.contains("pub next: Option<Box<Node>>,"), "{actual}");
        assert!(actual.contains("pub parent: Box<Node>,"), "{actual}");
        assert!(actual.contains("pub children: Vec<Node>,"), "{actual}");
        assert!(actual.contains("pub leaf: Option<Leaf>,"), "{actual}");
    }

    const COLLIDING_NAMES_SDL: &str = r#"
        enum Mode {
          fooBar
          FOO_BAR
        }

        type Example {
          fooBar: String
          foo_bar: Int
          foo_bar_2: Boolean
          type: String
          Type: Int
          mode: Mode
        }

        type Thing {
          a: Int
        }

        type _Thing {
          b: Int
        }

        type foo_bar {
          c: Int
        }

        type FooBar {
          d: Int
        }

        union Either = foo_bar | FooBar

        type Query {
          find(fooBar: Int, foo_bar: String): Example
        }
    "#;

    #[test]
    fn colliding_names_compile_with_their_codecs() {
        compile_models_and_codecs("colliding_names", COLLIDING_NAMES_SDL);
    }

    #[test]
    fn colliding_names_are_disambiguated_in_order_keeping_wire_names() {
        let ir = lower_schema_sdl(COLLIDING_NAMES_SDL).expect("schema should lower");
        let operations =
            list_schema_operations_sdl(COLLIDING_NAMES_SDL).expect("operations should resolve");

        let actual = emit_rust_with_operations(&ir, &operations);
        let file = syn::parse_file(&actual).expect("generated Rust should parse");
        let mut seen = BTreeSet::new();
        for item in &file.items {
            if let syn::Item::Struct(item) = item {
                assert!(seen.insert(item.ident.to_string()), "{actual}");
            }
            if let syn::Item::Enum(item) = item {
                assert!(seen.insert(item.ident.to_string()), "{actual}");
            }
        }

        // A field keeps its own name when that name is free; a later field
        // that normalises to a taken name gets the first free `_N` suffix,
        // and its wire name stays in `serde(rename)`.
        assert!(
            actual.contains("#[serde(rename = \"fooBar\")]\n    pub foo_bar: Option<String>,"),
            "{actual}"
        );
        assert!(
            actual.contains("#[serde(rename = \"foo_bar\")]\n    pub foo_bar_3: Option<i32>,"),
            "{actual}"
        );
        assert!(
            actual.contains("    pub foo_bar_2: Option<bool>,"),
            "{actual}"
        );
        assert!(
            actual.contains("    pub r#type: Option<String>,"),
            "{actual}"
        );
        assert!(
            actual.contains("#[serde(rename = \"Type\")]\n    pub type_2: Option<i32>,"),
            "{actual}"
        );
        assert!(
            actual.contains("#[serde(rename = \"FOO_BAR\")]\n    FooBar2,"),
            "{actual}"
        );
        assert!(actual.contains("pub struct Thing {"), "{actual}");
        assert!(actual.contains("pub struct Thing2 {"), "{actual}");
        assert!(actual.contains("    pub mode: Option<Mode>,"), "{actual}");
        assert!(
            actual.contains("#[serde(rename = \"foo_bar\")]\n    pub foo_bar_2: Option<String>,"),
            "{actual}"
        );
    }

    #[test]
    fn golden_codec_schema_compiles_with_its_models() {
        compile_models_and_codecs(
            "golden_codec_schema",
            include_str!("../../../test/fixtures/typescript-emitter/le-binary-codec.graphql"),
        );
    }

    #[test]
    fn float_codecs_narrow_on_write_and_widen_on_read_explicitly() {
        let sdl = r#"
            input Scale {
              ratio: Float!
              maybeRatio: Float
            }
        "#;
        let ir = lower_schema_sdl(sdl).expect("schema should lower");
        let codecs = emit_le_binary_rust(&ir, &[], LE_BINARY_RUST_DEFAULT_CODEC_IMPORT);

        assert!(
            codecs.contains("writer.write_f32_le(value.ratio as f32);"),
            "{codecs}"
        );
        assert!(
            codecs.contains("writer.write_option(&value.maybe_ratio, |writer, x| writer.write_f32_le(*x as f32));"),
            "{codecs}"
        );
        assert!(
            codecs.contains("ratio: reader.read_f32_le().map(f64::from)?,"),
            "{codecs}"
        );
        assert!(
            codecs.contains(
                "maybe_ratio: reader.read_option(|reader| reader.read_f32_le().map(f64::from))?,"
            ),
            "{codecs}"
        );
    }

    fn rust_operation_impl_block<'a>(actual: &'a str, request_name: &str) -> &'a str {
        let marker = format!("impl {request_name} {{");
        let start = actual
            .find(&marker)
            .expect("operation impl block should exist");
        let tail = &actual[start..];
        let end = tail
            .find("\n}\n")
            .expect("operation impl block should close")
            + "\n}\n".len();
        &tail[..end]
    }

    fn hostile_variant_validator_source() -> String {
        let validator = RustVariantValidator {
            function_name: "validate_playback_mode_input_variant".to_string(),
            input_type: "PlaybackModeInput".to_string(),
            discriminator_field: "kind".to_string(),
            enum_type: "PlaybackModeKind".to_string(),
            cases: vec![RustVariantValidatorCase {
                enum_variant: "Seek".to_string(),
                case_value: "CASE \"quote\" \\slash\nline\rreturn\t tab".to_string(),
                requires: vec!["target".to_string()],
                forbids: vec!["then".to_string()],
            }],
        };
        let mut actual = String::new();
        print_variant_validator(&mut actual, &validator);
        actual
    }

    fn compile_rust_source(test_name: &str, source: &str) {
        let dir = temporary_compile_dir(test_name);
        fs::create_dir_all(&dir).expect("temporary compile directory should create");
        let source_path = dir.join("lib.rs");
        fs::write(&source_path, source).expect("temporary Rust source should write");

        let output = Command::new("rustc")
            .arg("--edition=2021")
            .arg("--crate-type=lib")
            .arg(&source_path)
            .arg("--out-dir")
            .arg(&dir)
            .output()
            .expect("rustc should run");

        let _ = fs::remove_dir_all(&dir);

        assert!(
            output.status.success(),
            "generated Rust should compile\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Emits models (with operation bindings) and LE-binary codecs for `sdl`
    /// and compiles them together with `rustc`, offline: `serde` is a
    /// proc-macro stub and the codec runtime is a minimal fixture.
    fn compile_models_and_codecs(test_name: &str, sdl: &str) {
        let ir = lower_schema_sdl(sdl).expect("schema should lower");
        let operations = list_schema_operations_sdl(sdl).expect("operations should resolve");
        let models = emit_rust_with_operations(&ir, &operations);
        let codecs = emit_le_binary_rust(&ir, &operations, LE_BINARY_RUST_DEFAULT_CODEC_IMPORT);
        compile_generated_files(test_name, &models, &codecs);
    }

    /// Compiles generated `models` and `codecs` with `rustc` inside the
    /// fixture harness crate.
    fn compile_generated_files(test_name: &str, models: &str, codecs: &str) {
        let harness = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../test/fixtures/rust-emitter/compile-harness");
        let dir = temporary_compile_dir(test_name);
        fs::create_dir_all(&dir).expect("temporary compile directory should create");
        for file in ["lib.rs", "codec_runtime.rs", "serde.rs"] {
            fs::copy(harness.join(file), dir.join(file)).expect("harness file should copy");
        }
        fs::write(dir.join("models.rs"), models).expect("generated models should write");
        fs::write(dir.join("codecs.rs"), codecs).expect("generated codecs should write");

        let serde = Command::new("rustc")
            .args([
                "--edition=2021",
                "--crate-type=proc-macro",
                "--crate-name=serde",
            ])
            .arg(dir.join("serde.rs"))
            .arg("--out-dir")
            .arg(&dir)
            .output()
            .expect("rustc should run");
        assert!(
            serde.status.success(),
            "serde stub should compile\nstderr:\n{}",
            String::from_utf8_lossy(&serde.stderr)
        );
        let serde_lib = dir.join(format!(
            "{}serde{}",
            std::env::consts::DLL_PREFIX,
            std::env::consts::DLL_SUFFIX
        ));

        let output = Command::new("rustc")
            .args([
                "--edition=2021",
                "--crate-type=lib",
                "--crate-name=generated",
            ])
            .arg("--extern")
            .arg(format!("serde={}", serde_lib.display()))
            .arg(dir.join("lib.rs"))
            .arg("--out-dir")
            .arg(&dir)
            .output()
            .expect("rustc should run");

        let _ = fs::remove_dir_all(&dir);

        assert!(
            output.status.success(),
            "generated models and codecs should compile\nmodels:\n{models}\ncodecs:\n{codecs}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn cargo_check_generated_rust_crate(test_name: &str, source: &str) {
        let dir = temporary_compile_dir(test_name);
        let src_dir = dir.join("src");
        fs::create_dir_all(&src_dir).expect("temporary crate source directory should create");
        fs::write(
            dir.join("Cargo.toml"),
            r#"[package]
name = "wesley-generated-rust-check"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
serde = { version = "1", features = ["derive"] }
"#,
        )
        .expect("temporary crate manifest should write");
        fs::write(src_dir.join("lib.rs"), source).expect("temporary generated Rust should write");

        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let output = Command::new(cargo)
            .arg("check")
            .arg("--quiet")
            .arg("--manifest-path")
            .arg(dir.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", dir.join("target"))
            .output()
            .expect("cargo check should run");

        let _ = fs::remove_dir_all(&dir);

        assert!(
            output.status.success(),
            "generated Rust crate should pass cargo check\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn temporary_compile_dir(test_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "wesley-emit-rust-{test_name}-{}-{unique}",
            std::process::id()
        ))
    }
}
