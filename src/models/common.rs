//! Common types and enums used across the API.

use serde::Deserialize;
use std::fmt;

/// Supported languages for metadata content.
#[derive(Debug, Deserialize, Clone, Copy)]
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
impl fmt::Display for MetadataLanguage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MetadataLanguage::English => write!(f, "english"),
            MetadataLanguage::Arabic => write!(f, "arabic"),
        }
    }
}
