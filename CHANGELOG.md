# Changelog

All notable changes to this project are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- `crates/wesley-emit-rust`, Wesley's existing emitter, moved from flyingrobots/wesley at `bfff0f9a` (0.3.0-alpha.4) with its history and the fixtures its tests read. It depends on the published `wesley-core` and `wesley-emit-codec` 0.3.0-alpha.4 (flyingrobots/wesley#882).
- The repository, its license, and the rules for how the target generates code (flyingrobots/parley#116, flyingrobots/wesley#882).
- `crates/wesley-rust-syntax` 0.1.0, generic helpers for generating Rust with `syn`, `quote` and `prettyplease`: `naming::{type_name, field_name, variant_name, snake_case}`, `ident` (raw for `r#keyword`), `doc_attrs` and `inner_doc_attrs` (printed as `///` and `//!`), `print_file` (exactly one trailing newline) and `unique_names` (deterministic de-duplication). Its naming functions are `wesley-emit-rust`'s, moved rather than rewritten. Its tests compile a golden model with `rustc` offline against stand-ins for `serde` and `serde_json`, and fail if a string literal under its `src/` holds Rust syntax.

### Changed

- `wesley-emit-rust` takes its names from `wesley-rust-syntax`. Output is unchanged for every fixture; it differs only for names the old helpers got wrong or left non-idiomatic. A field named `self`, `super` or `crate` becomes `self_` (it was `r#self`, which is not a Rust token). Acronyms split as words (`userID` gives `user_id`, was `user_i_d`; `HTTPServer` gives `http_server`). A type name with an interior underscore is PascalCase (`foo_bar` gives `FooBar`, was `Foo_bar`). A variant mixing case and underscores keeps its humps (`camelCase_x` gives `CamelCaseX`, was `CamelcaseX`). Names that GraphQL cannot write (hyphens, a leading digit) follow the new crate's rules. Any renamed field keeps its wire name in `#[serde(rename)]`.
