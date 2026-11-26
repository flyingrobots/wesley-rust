# wesley-rust-syntax

Generic helpers for programs that generate Rust by building a `syn` tree (with `quote`) and printing it with `prettyplease`. It understands no schema and no directive; it supplies the small pieces every such generator needs. Nothing in it builds Rust source from strings, and a test fails if a string literal under `src/` holds Rust syntax.

```rust
use quote::quote;
use wesley_rust_syntax::naming::{field_name, type_name, variant_name};
use wesley_rust_syntax::{doc_attrs, ident, inner_doc_attrs, print_file};

let name = ident(&type_name("run-summary")); // RunSummary
let field = ident(&field_name("type")); // r#type
let docs = doc_attrs("What a run did.");
let mut file: syn::File = syn::parse2(quote! {
    #(#docs)*
    pub struct #name { pub #field: String }
})?;
file.attrs = inner_doc_attrs("Generated. Do not edit.");
let source = print_file(&file);
```

| Item | What it gives |
| --- | --- |
| `naming::type_name` | PascalCase; each word keeps its case after the first letter (`UserFilter`, `URL`), `run-summary` gives `RunSummary`; a leading digit is prefixed with `T`, `Self` becomes `SelfType` |
| `naming::field_name` | snake_case, splitting acronyms (`HTTPServer` gives `http_server`) and dropping edge underscores (`_schema` gives `schema`); keywords stay raw (`r#type`), and `self`, `super`, `crate`, `Self` get a trailing underscore |
| `naming::variant_name` | PascalCase for a wire value, lowercasing shouted words: `what-ran` gives `WhatRan`, `READ_ONLY` gives `ReadOnly`, `none` gives `None`; a leading digit is prefixed with `V` |
| `naming::snake_case` | plain snake_case with no keyword handling, for identifier stems |
| `ident` | a `syn::Ident`, raw for an `r#keyword` name |
| `doc_attrs`, `inner_doc_attrs` | one `#[doc]` or `#![doc]` per line, printed by `prettyplease` as `///` or `//!` |
| `print_file` | `prettyplease::unparse`, ending in exactly one newline |
| `unique_names` | deterministic de-duplication of normalised names: in order, a free form is kept and a taken one gets the first free numeric suffix (`foo_bar_2`, `Thing2`) |

Identifiers are ASCII: a word is a run of ASCII letters and digits, and any other character, including a non-ASCII letter, is a word break. Every function returns a valid identifier for any input, and the same input always gives the same name.

Apache-2.0.
