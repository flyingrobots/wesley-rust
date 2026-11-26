//! Golden model for wesley-rust-syntax.
//!
//! Do not edit.
/// What a run did.
///
/// Carries unknown members through `extra`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RunSummary {
    #[serde(rename = "_schema")]
    pub schema: String,
    pub r#type: String,
    pub dry_run: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
    #[serde(rename = "what-ran")]
    pub what_ran: WhatRan,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
/// Which part of the work ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WhatRan {
    #[serde(rename = "count-only")]
    CountOnly,
    #[serde(rename = "none")]
    None,
    #[serde(rename = "all")]
    All,
}
