#![deny(warnings)]
#![deny(missing_docs)]

//! Generic helpers for programs that generate Rust with `syn`, `quote` and
//! `prettyplease`.
//!
//! Generated code is built as a `syn` tree and printed by `prettyplease`;
//! nothing here builds Rust source from strings. This crate supplies the
//! pieces every such generator needs and none of them define: turning foreign
//! names into Rust identifiers, documentation attributes, and printing a file.

use std::collections::BTreeSet;

use proc_macro2::Span;
use syn::parse_quote;

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
    match name.strip_prefix("r#") {
        Some(raw) => syn::Ident::new_raw(raw, Span::call_site()),
        None => syn::Ident::new(name, Span::call_site()),
    }
}

/// Outer documentation attributes for `text`, one `#[doc = " line"]` per
/// line, which `prettyplease` prints as `/// line`.
///
/// Lines end at `\n`, `\r\n` or a lone `\r` (a bare carriage return is not
/// allowed in a doc comment). A non-empty line gains the one leading space a
/// `///` comment has; an empty line stays empty. Empty `text` gives no
/// attributes.
#[must_use]
pub fn doc_attrs(text: &str) -> Vec<syn::Attribute> {
    doc_lines(text)
        .map(|line| parse_quote!(#[doc = #line]))
        .collect()
}

/// Inner documentation attributes for `text`, one `#![doc = " line"]` per
/// line, which `prettyplease` prints as `//! line`; for a file header. Lines
/// are split as in [`doc_attrs`].
#[must_use]
pub fn inner_doc_attrs(text: &str) -> Vec<syn::Attribute> {
    doc_lines(text)
        .map(|line| parse_quote!(#![doc = #line]))
        .collect()
}

fn doc_lines(text: &str) -> impl Iterator<Item = String> + '_ {
    text.lines().flat_map(|line| line.split('\r')).map(|line| {
        if line.is_empty() {
            String::new()
        } else {
            let mut value = String::with_capacity(line.len() + 1);
            value.push(' ');
            value.push_str(line);
            value
        }
    })
}

/// Prints `file` with `prettyplease`, ending in exactly one newline.
#[must_use]
pub fn print_file(file: &syn::File) -> String {
    let mut printed = prettyplease::unparse(file);
    printed.truncate(printed.trim_end_matches('\n').len());
    printed.push('\n');
    printed
}

/// Normalises each of `names` with `normalise` and makes the results unique,
/// deterministically.
///
/// Names are taken in order. A name whose normalised form is not yet taken
/// keeps it. A later name whose form is taken gets the first suffix `N`
/// (counting from 2) that gives a form neither taken nor the unsuffixed form
/// of any name in the list, so a name that is unique on its own never loses
/// its form to a suffixed one. The suffix is appended directly to a
/// PascalCase form (`Thing2`) and after an underscore otherwise
/// (`foo_bar_2`), and the result is normalised again, so a second `r#type`
/// becomes `type_2`.
#[must_use]
pub fn unique_names(names: &[String], normalise: fn(&str) -> String) -> Vec<String> {
    let forms = names.iter().map(|name| normalise(name)).collect::<Vec<_>>();
    let reserved = forms.iter().cloned().collect::<BTreeSet<_>>();
    let mut taken = BTreeSet::new();

    forms
        .into_iter()
        .map(|form| {
            if taken.insert(form.clone()) {
                return form;
            }
            let stem = form.strip_prefix("r#").unwrap_or(&form);
            let separator = if stem.starts_with(|ch: char| ch.is_ascii_uppercase()) {
                ""
            } else {
                "_"
            };
            let chosen = (2_u64..)
                .map(|suffix| normalise(&format!("{stem}{separator}{suffix}")))
                .find(|candidate| !reserved.contains(candidate) && !taken.contains(candidate))
                .expect("an unbounded suffix range always yields a free name");
            taken.insert(chosen.clone());
            chosen
        })
        .collect()
}
