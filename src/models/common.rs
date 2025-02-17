//! Common types and enums used across the API.

use serde::Deserialize;

/// Supported languages for metadata content.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetadataLanguage {
    English,
    Arabic,
}

/// Supported browser profiles for hard to archive sites
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BrowserProfile {
    Facebook,
}
