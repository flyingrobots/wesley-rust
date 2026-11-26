//! Documentation attributes print as `///` and `//!` and survive a round trip
//! through `prettyplease` and `syn`; `print_file` ends in one newline.

use quote::quote;
use wesley_rust_syntax::{doc_attrs, ident, inner_doc_attrs, print_file};

fn doc_values(attrs: &[syn::Attribute]) -> Vec<String> {
    attrs
        .iter()
        .map(|attr| {
            let syn::Meta::NameValue(meta) = &attr.meta else {
                panic!("doc attribute should be name = value");
            };
            assert!(meta.path.is_ident("doc"));
            let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(value),
                ..
            }) = &meta.value
            else {
                panic!("doc attribute should hold a string");
            };
            value.value()
        })
        .collect()
}

#[test]
fn doc_attrs_are_one_outer_attribute_per_line() {
    let attrs = doc_attrs("First line.\n\nSecond line.");
    assert!(attrs
        .iter()
        .all(|attr| matches!(attr.style, syn::AttrStyle::Outer)));
    assert_eq!(doc_values(&attrs), [" First line.", "", " Second line."]);
}

#[test]
fn inner_doc_attrs_are_one_inner_attribute_per_line() {
    let attrs = inner_doc_attrs("Generated.\nDo not edit.");
    assert!(attrs
        .iter()
        .all(|attr| matches!(attr.style, syn::AttrStyle::Inner(_))));
    assert_eq!(doc_values(&attrs), [" Generated.", " Do not edit."]);
}

#[test]
fn doc_attrs_split_carriage_returns_and_keep_quotes() {
    assert_eq!(
        doc_values(&doc_attrs("a \"quoted\" line\r\nnext\rlast")),
        [" a \"quoted\" line", " next", " last"]
    );
}

#[test]
fn doc_attrs_print_as_line_comments_and_round_trip() {
    let name = ident("Example");
    let docs = doc_attrs("First line.\n\nSecond line with `code` and \"quotes\".");
    let mut file: syn::File = syn::parse2(quote! {
        #(#docs)*
        pub struct #name;
    })
    .expect("file should build");
    file.attrs = inner_doc_attrs("Header line.\n\nMore header.");

    let printed = print_file(&file);

    assert!(
        printed.starts_with("//! Header line.\n//!\n//! More header.\n"),
        "{printed}"
    );
    assert!(
        printed.contains(
            "/// First line.\n///\n/// Second line with `code` and \"quotes\".\npub struct Example;\n"
        ),
        "{printed}"
    );

    let reparsed = syn::parse_file(&printed).expect("printed file should parse");
    assert_eq!(doc_values(&reparsed.attrs), doc_values(&file.attrs));
    let syn::Item::Struct(item) = &reparsed.items[0] else {
        panic!("first item should be the struct");
    };
    assert_eq!(doc_values(&item.attrs), doc_values(&docs));
}

#[test]
fn print_file_ends_in_exactly_one_newline() {
    let name = ident("Example");
    let file: syn::File = syn::parse2(quote! { pub struct #name; }).expect("file should build");
    let printed = print_file(&file);
    assert!(printed.ends_with(";\n"), "{printed:?}");
    assert!(!printed.ends_with("\n\n"), "{printed:?}");

    let empty = syn::File {
        shebang: None,
        attrs: Vec::new(),
        items: Vec::new(),
    };
    assert_eq!(print_file(&empty), "\n");
}
