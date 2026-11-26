#![deny(warnings)]
#![deny(missing_docs)]

//! Generic helpers for programs that generate Rust with `syn`, `quote` and
//! `prettyplease`.
//!
//! Generated code is built as a `syn` tree and printed by `prettyplease`;
//! nothing here builds Rust source from strings. This crate supplies the
//! pieces every such generator needs and none of them define: turning foreign
//! names into Rust identifiers, documentation attributes, and printing a file.

pub mod naming;

/// Builds an identifier from a name produced by [`naming`].
///
/// A name of the form `r#keyword` becomes a raw identifier.
///
/// # Panics
///
/// Panics if `name` is not a valid Rust identifier, as `proc_macro2::Ident`
/// does; every name [`naming`] returns is valid.
#[must_use]
pub fn ident(name: &str) -> syn::Ident {
    let _ = name;
    unimplemented!()
}

/// Outer documentation attributes for `text`, one `#[doc = " line"]` per
/// line, which `prettyplease` prints as `/// line`.
#[must_use]
pub fn doc_attrs(text: &str) -> Vec<syn::Attribute> {
    let _ = text;
    unimplemented!()
}

/// Inner documentation attributes for `text`, one `#![doc = " line"]` per
/// line, which `prettyplease` prints as `//! line`; for a file header.
#[must_use]
pub fn inner_doc_attrs(text: &str) -> Vec<syn::Attribute> {
    let _ = text;
    unimplemented!()
}

/// Prints `file` with `prettyplease`, ending in exactly one newline.
#[must_use]
pub fn print_file(file: &syn::File) -> String {
    let _ = file;
    unimplemented!()
}

/// Normalises each of `names` with `normalise` and makes the results unique,
/// deterministically.
#[must_use]
pub fn unique_names(names: &[String], normalise: fn(&str) -> String) -> Vec<String> {
    let _ = (names, normalise);
    unimplemented!()
}
