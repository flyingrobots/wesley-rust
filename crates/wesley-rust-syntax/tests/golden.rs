//! A small model built as a `syn` tree with this crate's helpers, printed by
//! `print_file`, compared to a golden file and compiled by `rustc` offline
//! against stand-ins for `serde` and `serde_json`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

use quote::quote;
use wesley_rust_syntax::naming::{field_name, type_name, variant_name};
use wesley_rust_syntax::{doc_attrs, ident, inner_doc_attrs, print_file};

fn model_file() -> syn::File {
    let summary = ident(&type_name("run-summary"));
    let summary_docs = doc_attrs("What a run did.\n\nCarries unknown members through `extra`.");
    let schema = ident(&field_name("_schema"));
    let schema_wire = "_schema";
    let kind = ident(&field_name("type"));
    let dry_run = ident(&field_name("dry_run"));
    let has_more = ident(&field_name("has_more"));
    let what_ran_field = ident(&field_name("what-ran"));
    let what_ran_wire = "what-ran";

    let what_ran = ident(&type_name("what-ran"));
    let what_ran_docs = doc_attrs("Which part of the work ran.");
    let wires = ["count-only", "none", "all"];
    let variants = wires.iter().map(|wire| ident(&variant_name(wire)));

    let mut file: syn::File = syn::parse2(quote! {
        #(#summary_docs)*
        #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
        pub struct #summary {
            #[serde(rename = #schema_wire)]
            pub #schema: String,
            pub #kind: String,
            pub #dry_run: bool,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub #has_more: Option<bool>,
            #[serde(rename = #what_ran_wire)]
            pub #what_ran_field: #what_ran,
            #[serde(flatten)]
            pub extra: serde_json::Map<String, serde_json::Value>,
        }

        #(#what_ran_docs)*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        pub enum #what_ran {
            #(
                #[serde(rename = #wires)]
                #variants,
            )*
        }
    })
    .expect("model should build");
    file.attrs = inner_doc_attrs("Golden model for wesley-rust-syntax.\n\nDo not edit.");
    file
}

#[test]
fn golden_model_prints_byte_for_byte() {
    let printed = print_file(&model_file());
    let expected = include_str!("golden/model.rs");
    assert_eq!(printed, expected, "printed:\n{printed}");
}

#[test]
fn golden_model_compiles() {
    let dir = temporary_dir("golden_model");
    fs::create_dir_all(&dir).expect("temporary directory should create");
    let stubs = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/stubs");

    rustc(
        &dir,
        &["--crate-type=proc-macro", "--crate-name=serde"],
        &stubs.join("serde.rs"),
        &[],
    );
    rustc(
        &dir,
        &["--crate-type=rlib", "--crate-name=serde_json"],
        &stubs.join("serde_json.rs"),
        &[],
    );

    let model = dir.join("model.rs");
    fs::write(&model, print_file(&model_file())).expect("model should write");
    let serde = dir.join(format!(
        "{}serde{}",
        env::consts::DLL_PREFIX,
        env::consts::DLL_SUFFIX
    ));
    let serde_json = dir.join("libserde_json.rlib");
    rustc(
        &dir,
        &["--crate-type=lib", "--crate-name=golden", "-Dwarnings"],
        &model,
        &[("serde", &serde), ("serde_json", &serde_json)],
    );

    let _ = fs::remove_dir_all(&dir);
}

fn rustc(dir: &Path, args: &[&str], source: &Path, externs: &[(&str, &Path)]) {
    let mut command = Command::new("rustc");
    command.arg("--edition=2021").args(args).arg(source);
    for (name, path) in externs {
        command
            .arg("--extern")
            .arg(format!("{name}={}", path.display()));
    }
    let output = command
        .arg("--out-dir")
        .arg(dir)
        .output()
        .expect("rustc should run");
    assert!(
        output.status.success(),
        "rustc {} should succeed\nstderr:\n{}",
        source.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn temporary_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after the epoch")
        .as_nanos();
    env::temp_dir().join(format!(
        "wesley-rust-syntax-{name}-{}-{unique}",
        std::process::id()
    ))
}
