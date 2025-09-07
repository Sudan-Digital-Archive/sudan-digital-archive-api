//! Common types and enums used across the API.

use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;

/// Supported languages for metadata content.
#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MetadataLanguage {
    #[default]
    English,
    Arabic,
}

/// Supported browser profiles for hard to archive sites
#[derive(Clone, Debug, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum BrowserProfile {
    Facebook,
}

/// Display implementation for MetadataLanguage. Mostly exists
/// for string interpolation, logging and debugging.
impl fmt::Display for MetadataLanguage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MetadataLanguage::English => write!(f, "en"),
            MetadataLanguage::Arabic => write!(f, "ar"),
        }
    }
}
