//! An offline stand-in for `serde`'s derive macros, so generated models can be
//! compiled by `rustc` in tests without fetching crates. Each derive accepts the
//! `#[serde(...)]` helper attribute and expands to nothing.

extern crate proc_macro;

use proc_macro::TokenStream;

/// Accepts `#[derive(serde::Serialize)]` and expands to nothing.
#[proc_macro_derive(Serialize, attributes(serde))]
pub fn serialize(_item: TokenStream) -> TokenStream {
    TokenStream::new()
}

/// Accepts `#[derive(serde::Deserialize)]` and expands to nothing.
#[proc_macro_derive(Deserialize, attributes(serde))]
pub fn deserialize(_item: TokenStream) -> TokenStream {
    TokenStream::new()
}
