//! An offline stand-in for the parts of `serde_json` the golden model names,
//! so it can be compiled by `rustc` without fetching crates.

/// A JSON object, as `serde_json::Map<String, Value>`.
pub type Map<K, V> = std::collections::BTreeMap<K, V>;

/// A JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `null`.
    Null,
    /// A boolean.
    Bool(bool),
    /// A number, held as text.
    Number(String),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Value>),
    /// An object.
    Object(Map<String, Value>),
}
