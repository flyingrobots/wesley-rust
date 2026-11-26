//! Naming edge cases: keywords, leading underscores, separators, digits,
//! acronyms and non-ASCII input.

use wesley_rust_syntax::naming::{field_name, snake_case, type_name, variant_name};
use wesley_rust_syntax::{ident, unique_names};

#[test]
fn type_names_are_pascal_case() {
    assert_eq!(type_name("user"), "User");
    assert_eq!(type_name("UserFilter"), "UserFilter");
    assert_eq!(type_name("run-summary"), "RunSummary");
    assert_eq!(type_name("foo_bar"), "FooBar");
    assert_eq!(type_name("parley.run summary/v1"), "ParleyRunSummaryV1");
    assert_eq!(type_name("URL"), "URL");
    assert_eq!(type_name("_Thing"), "Thing");
    assert_eq!(type_name("DateTime"), "DateTime");
}

#[test]
fn type_names_avoid_keywords_digits_and_emptiness() {
    assert_eq!(type_name("Self"), "SelfType");
    assert_eq!(type_name("self"), "SelfType");
    assert_eq!(type_name("2d-point"), "T2dPoint");
    assert_eq!(type_name(""), "GeneratedType");
    assert_eq!(type_name("--"), "GeneratedType");
}

#[test]
fn field_names_are_snake_case() {
    assert_eq!(field_name("displayName"), "display_name");
    assert_eq!(field_name("dry_run"), "dry_run");
    assert_eq!(field_name("has_more"), "has_more");
    assert_eq!(field_name("hasMore"), "has_more");
    assert_eq!(field_name("_schema"), "schema");
    assert_eq!(field_name("what-ran"), "what_ran");
    assert_eq!(field_name("userID"), "user_id");
    assert_eq!(field_name("HTTPServer"), "http_server");
    assert_eq!(field_name("dataBase64"), "data_base64");
}

#[test]
fn field_names_keep_keywords_raw_where_rust_allows() {
    assert_eq!(field_name("type"), "r#type");
    assert_eq!(field_name("Type"), "r#type");
    assert_eq!(field_name("async"), "r#async");
    assert_eq!(field_name("gen"), "r#gen");
    assert_eq!(field_name("match"), "r#match");
}

#[test]
fn field_names_that_cannot_be_raw_get_a_trailing_underscore() {
    assert_eq!(field_name("self"), "self_");
    assert_eq!(field_name("Self"), "self_");
    assert_eq!(field_name("super"), "super_");
    assert_eq!(field_name("crate"), "crate_");
}

#[test]
fn field_names_avoid_digits_and_emptiness() {
    assert_eq!(field_name("2fa"), "_2fa");
    assert_eq!(field_name(""), "generated_field");
    assert_eq!(field_name("_"), "generated_field");
}

#[test]
fn variant_names_are_pascal_case_from_wire_values() {
    assert_eq!(variant_name("what-ran"), "WhatRan");
    assert_eq!(variant_name("count-only"), "CountOnly");
    assert_eq!(variant_name("none"), "None");
    assert_eq!(variant_name("READ_ONLY"), "ReadOnly");
    assert_eq!(variant_name("ADMIN"), "Admin");
    assert_eq!(variant_name("fooBar"), "FooBar");
    assert_eq!(variant_name("XMLHttp"), "XMLHttp");
}

#[test]
fn variant_names_avoid_keywords_digits_and_emptiness() {
    assert_eq!(variant_name("2d"), "V2d");
    assert_eq!(variant_name("404-not-found"), "V404NotFound");
    assert_eq!(variant_name("Self"), "SelfVariant");
    assert_eq!(variant_name(""), "GeneratedVariant");
}

#[test]
fn non_ascii_characters_are_word_breaks() {
    assert_eq!(type_name("naïve-time"), "NaVeTime");
    assert_eq!(field_name("naïveTime"), "na_ve_time");
    assert_eq!(variant_name("über"), "Ber");
    assert_eq!(field_name("日本"), "generated_field");
}

#[test]
fn snake_case_has_no_keyword_handling() {
    assert_eq!(
        snake_case("MutationMakeWidgetRequest"),
        "mutation_make_widget_request"
    );
    assert_eq!(snake_case("type"), "type");
    assert_eq!(snake_case("_schema"), "schema");
}

#[test]
fn every_name_is_a_valid_identifier() {
    for name in [
        "type", "self", "Self", "super", "crate", "_", "", "2fa", "what-ran", "naïve", "gen",
    ] {
        let _ = ident(&type_name(name));
        let _ = ident(&field_name(name));
        let _ = ident(&variant_name(name));
    }
}

#[test]
fn ident_is_raw_only_for_raw_names() {
    let raw = ident("r#type");
    assert_eq!(raw.to_string(), "r#type");
    assert_eq!(syn::parse_str::<syn::Ident>("r#type").unwrap(), raw);

    let plain = ident("display_name");
    assert_eq!(plain.to_string(), "display_name");
}

fn strings(names: &[&str]) -> Vec<String> {
    names.iter().map(ToString::to_string).collect()
}

#[test]
fn unique_names_keeps_free_forms_and_suffixes_later_duplicates() {
    assert_eq!(
        unique_names(
            &strings(&["fooBar", "foo_bar", "foo_bar_2", "type", "Type"]),
            field_name
        ),
        strings(&["foo_bar", "foo_bar_3", "foo_bar_2", "r#type", "type_2"])
    );
    assert_eq!(
        unique_names(&strings(&["Thing", "_Thing", "Thing_"]), type_name),
        strings(&["Thing", "Thing2", "Thing3"])
    );
    assert_eq!(
        unique_names(&strings(&["fooBar", "FOO_BAR"]), variant_name),
        strings(&["FooBar", "FooBar2"])
    );
}

#[test]
fn unique_names_leaves_distinct_names_alone() {
    assert_eq!(
        unique_names(&strings(&["id", "displayName", "createdAt"]), field_name),
        strings(&["id", "display_name", "created_at"])
    );
}
