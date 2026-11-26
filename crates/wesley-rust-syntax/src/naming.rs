//! Rust names for foreign names (GraphQL fields, JSON members, wire values).
//!
//! Every function returns a valid Rust identifier for any input, and the same
//! input always gives the same name. Identifiers are ASCII: a word is a run of
//! ASCII letters and digits, and any other character (`_`, `-`, `.`, a space,
//! or a non-ASCII letter) is a word break and is dropped. Two different names
//! can give the same identifier; [`crate::unique_names`] makes a list of them
//! unique.
//!
//! These functions began as `wesley-emit-rust`'s private naming helpers and
//! were moved here so that every generator shares them.

/// A PascalCase type name.
///
/// Each word starts with an uppercase letter and keeps the rest of its case,
/// so `UserFilter`, `URL` and `DateTime` are unchanged and `run-summary`
/// becomes `RunSummary`. A name that would start with a digit is prefixed
/// with `T`, `Self` becomes `SelfType`, and a name with no words becomes
/// `GeneratedType`.
#[must_use]
pub fn type_name(name: &str) -> String {
    let mut candidate = words(name)
        .map(|word| capitalise(word, false))
        .collect::<String>();
    if candidate.is_empty() {
        candidate.push_str("GeneratedType");
    }
    if starts_with_digit(&candidate) {
        candidate.insert(0, 'T');
    }
    if is_reserved_word(&candidate) {
        candidate.push_str("Type");
    }

    candidate
}

/// A snake_case field name.
///
/// Words are split at separators and at case changes (`hasMore` and
/// `has_more` both give `has_more`, `HTTPServer` gives `http_server`), and
/// leading and trailing underscores are dropped (`_schema` gives `schema`). A
/// Rust keyword is kept as a raw identifier (`r#type`), except `self`,
/// `super`, `crate` and `Self`, which cannot be raw and get a trailing
/// underscore (`self_`). A name that would start with a digit is prefixed
/// with `_`, and a name with no words becomes `generated_field`.
#[must_use]
pub fn field_name(name: &str) -> String {
    let mut candidate = snake_case(name);
    if candidate.is_empty() {
        candidate.push_str("generated_field");
    }
    if starts_with_digit(&candidate) {
        candidate.insert(0, '_');
    }
    if cannot_be_raw(&candidate) {
        candidate.push('_');
    } else if is_reserved_word(&candidate) {
        candidate.insert_str(0, "r#");
    }

    candidate
}

/// A PascalCase enum variant name for a wire value.
///
/// Like [`type_name`], except that a word with no lowercase letter is
/// lowercased after its first letter, so `READ_ONLY` becomes `ReadOnly`:
/// `what-ran` gives `WhatRan`, `count-only` gives `CountOnly`, and `none`
/// gives `None`. A name that would start with a digit is prefixed with `V`,
/// `Self` becomes `SelfVariant`, and a name with no words becomes
/// `GeneratedVariant`.
#[must_use]
pub fn variant_name(wire: &str) -> String {
    let mut candidate = words(wire)
        .map(|word| capitalise(word, true))
        .collect::<String>();
    if candidate.is_empty() {
        candidate.push_str("GeneratedVariant");
    }
    if starts_with_digit(&candidate) {
        candidate.insert(0, 'V');
    }
    if is_reserved_word(&candidate) {
        candidate.push_str("Variant");
    }

    candidate
}

/// A plain snake_case form of `name`, with no keyword handling; for building
/// a longer identifier such as a function name stem.
///
/// An underscore goes before an uppercase letter that follows a lowercase
/// letter or a digit, or that ends a run of capitals before a lowercase
/// letter; every other character outside ASCII letters and digits becomes
/// one underscore; leading and trailing underscores are dropped.
#[must_use]
pub fn snake_case(name: &str) -> String {
    let chars = name.chars().collect::<Vec<_>>();
    let mut out = String::new();

    for (index, &ch) in chars.iter().enumerate() {
        if ch.is_ascii_uppercase() {
            let previous = index.checked_sub(1).map(|previous| chars[previous]);
            let next = chars.get(index + 1).copied();
            let after_word = previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit());
            let ends_acronym = previous.is_some_and(|p| p.is_ascii_uppercase())
                && next.is_some_and(|n| n.is_ascii_lowercase());
            if (after_word || ends_acronym) && !out.ends_with('_') {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            out.push(ch);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }

    out.trim_matches('_').to_string()
}

/// The runs of ASCII letters and digits in `name`.
fn words(name: &str) -> impl Iterator<Item = &str> {
    name.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
}

/// `word` with its first letter uppercased; with `lower_shouting`, a word
/// with no lowercase letter is lowercased after its first letter.
fn capitalise(word: &str, lower_shouting: bool) -> String {
    let shouting = lower_shouting && !word.chars().any(|ch| ch.is_ascii_lowercase());
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::with_capacity(word.len());
    out.push(first.to_ascii_uppercase());
    if shouting {
        out.extend(chars.map(|ch| ch.to_ascii_lowercase()));
    } else {
        out.extend(chars);
    }
    out
}

fn starts_with_digit(name: &str) -> bool {
    name.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

/// Keywords that cannot be written as raw identifiers.
fn cannot_be_raw(name: &str) -> bool {
    matches!(name, "self" | "super" | "crate" | "Self")
}

/// Strict, reserved and edition-dependent keywords of Rust.
fn is_reserved_word(name: &str) -> bool {
    matches!(
        name,
        "Self"
            | "abstract"
            | "as"
            | "async"
            | "await"
            | "become"
            | "box"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "do"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "final"
            | "fn"
            | "for"
            | "gen"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "macro"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "override"
            | "priv"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "try"
            | "type"
            | "typeof"
            | "union"
            | "unsafe"
            | "unsized"
            | "use"
            | "virtual"
            | "where"
            | "while"
            | "yield"
    )
}
