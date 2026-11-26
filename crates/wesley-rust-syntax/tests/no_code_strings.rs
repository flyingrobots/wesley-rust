//! A crude guard for the rule that generated code is built as a syntax tree:
//! no string literal under `src/` may contain Rust syntax. Doc comments are
//! not string literals and are skipped.

use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::{Delimiter, TokenStream, TokenTree};

/// Text that only appears in a string literal when the string is code.
const CODE_MARKERS: &[&str] = &["pub struct", "pub enum", "fn ", "impl ", "#[", "#![", "=>"];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("source directory should read") {
        let path = entry.expect("directory entry should read").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}

/// The string literals in `tokens` that contain a code marker, outside doc
/// attributes.
fn code_strings(tokens: TokenStream) -> Vec<String> {
    let mut found = Vec::new();
    for token in tokens {
        match token {
            TokenTree::Group(group) => {
                let is_doc = group.delimiter() == Delimiter::Bracket
                    && matches!(
                        group.stream().into_iter().next(),
                        Some(TokenTree::Ident(ident)) if ident == "doc"
                    );
                if !is_doc {
                    found.extend(code_strings(group.stream()));
                }
            }
            TokenTree::Literal(literal) => {
                if let syn::Lit::Str(value) = syn::Lit::new(literal) {
                    let value = value.value();
                    if CODE_MARKERS.iter().any(|marker| value.contains(marker)) {
                        found.push(value);
                    }
                }
            }
            TokenTree::Ident(_) | TokenTree::Punct(_) => {}
        }
    }
    found
}

#[test]
fn no_source_file_builds_rust_from_a_string() {
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(!files.is_empty(), "src/ should hold Rust files");

    for file in files {
        let source = fs::read_to_string(&file).expect("source should read");
        let tokens = source
            .parse::<TokenStream>()
            .expect("source should tokenize");
        let found = code_strings(tokens);
        assert!(
            found.is_empty(),
            "{} builds Rust from strings: {found:?}",
            file.display()
        );
    }
}

#[test]
fn the_guard_finds_code_in_a_string_and_ignores_doc_comments() {
    let sample = concat!(
        "/// pub struct Documented;\n",
        "fn build() -> String { String::from(\"pub struct Generated;\") }\n",
        "fn harmless() -> &'static str { \"a plain message\" }\n",
    );
    let tokens = sample
        .parse::<TokenStream>()
        .expect("sample should tokenize");
    assert_eq!(code_strings(tokens), ["pub struct Generated;"]);
}
