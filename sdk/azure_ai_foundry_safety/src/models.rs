//! Shared types for Azure AI Content Safety services.

use serde::{Deserialize, Serialize};

/// API version query parameter for Content Safety API requests.
pub(crate) const CONTENT_SAFETY_API_VERSION: &str = "api-version=2024-09-01";

/// Maximum text length for text analysis and protected material endpoints (Unicode code points).
pub(crate) const MAX_TEXT_LENGTH: usize = 10_000;

/// Maximum blocklist name length (characters).
pub(crate) const MAX_BLOCKLIST_NAME_LENGTH: usize = 64;

/// Maximum description length for blocklists and blocklist items (characters).
pub(crate) const MAX_DESCRIPTION_LENGTH: usize = 1_024;

/// Maximum blocklist item text length (characters).
pub(crate) const MAX_ITEM_TEXT_LENGTH: usize = 128;

// ---------------------------------------------------------------------------
// Harm categories
// ---------------------------------------------------------------------------

/// Content harm categories detected by the Content Safety API.
///
/// Used by both text and image analysis endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HarmCategory {
    /// Hateful content targeting identity groups.
    Hate,
    /// Self-harm related content.
    SelfHarm,
    /// Sexual content.
    Sexual,
    /// Violent content.
    Violence,
    /// An unknown category not yet supported by this SDK.
    #[serde(other)]
    Unknown,
}

impl HarmCategory {
    /// Returns the API string representation of this category.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hate => "Hate",
            Self::SelfHarm => "SelfHarm",
            Self::Sexual => "Sexual",
            Self::Violence => "Violence",
            Self::Unknown => "Unknown",
        }
    }
}

impl std::fmt::Display for HarmCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Output type for text content analysis severity levels.
///
/// Controls how many severity levels are returned: 4 (0, 2, 4, 6) or 8 (0–7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OutputType {
    /// Four severity levels: 0, 2, 4, 6.
    #[default]
    FourSeverityLevels,
    /// Eight severity levels: 0, 1, 2, 3, 4, 5, 6, 7.
    EightSeverityLevels,
}

impl OutputType {
    /// Returns the API string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FourSeverityLevels => "FourSeverityLevels",
            Self::EightSeverityLevels => "EightSeverityLevels",
        }
    }
}

impl std::fmt::Display for OutputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Output type for image content analysis severity levels.
///
/// Images only support four severity levels (0, 2, 4, 6).
///
/// This enum is marked `#[non_exhaustive]` because the Azure Content Safety API
/// may add new output type variants in future API versions.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ImageOutputType {
    /// Four severity levels: 0, 2, 4, 6.
    #[default]
    FourSeverityLevels,
}

impl ImageOutputType {
    /// Returns the API string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FourSeverityLevels => "FourSeverityLevels",
        }
    }
}

impl std::fmt::Display for ImageOutputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Category analysis (shared response type)
// ---------------------------------------------------------------------------

/// A single category analysis result with severity level.
///
/// Returned by both text and image analysis endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CategoryAnalysis {
    /// The harm category that was analyzed.
    pub category: HarmCategory,
    /// The severity level (0–7 for text with `EightSeverityLevels`, 0/2/4/6 otherwise).
    pub severity: u8,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- HarmCategory --

    #[test]
    fn test_harm_category_deserializes_hate() {
        let cat: HarmCategory = serde_json::from_str("\"Hate\"").unwrap();
        assert_eq!(cat, HarmCategory::Hate);
    }

    #[test]
    fn test_harm_category_deserializes_all_variants() {
        assert_eq!(
            serde_json::from_str::<HarmCategory>("\"SelfHarm\"").unwrap(),
            HarmCategory::SelfHarm
        );
        assert_eq!(
            serde_json::from_str::<HarmCategory>("\"Sexual\"").unwrap(),
            HarmCategory::Sexual
        );
        assert_eq!(
            serde_json::from_str::<HarmCategory>("\"Violence\"").unwrap(),
            HarmCategory::Violence
        );
    }

    #[test]
    fn test_harm_category_deserializes_unknown() {
        let cat: HarmCategory = serde_json::from_str("\"NewCategory\"").unwrap();
        assert_eq!(cat, HarmCategory::Unknown);
    }

    #[test]
    fn test_harm_category_display() {
        assert_eq!(format!("{}", HarmCategory::Hate), "Hate");
        assert_eq!(format!("{}", HarmCategory::SelfHarm), "SelfHarm");
        assert_eq!(format!("{}", HarmCategory::Sexual), "Sexual");
        assert_eq!(format!("{}", HarmCategory::Violence), "Violence");
        assert_eq!(format!("{}", HarmCategory::Unknown), "Unknown");
    }

    #[test]
    fn test_harm_category_as_str_matches_serde() {
        // Verify as_str matches what serde serializes to
        for cat in [
            HarmCategory::Hate,
            HarmCategory::SelfHarm,
            HarmCategory::Sexual,
            HarmCategory::Violence,
        ] {
            let serialized = serde_json::to_string(&cat).unwrap();
            let expected = format!("\"{}\"", cat.as_str());
            assert_eq!(serialized, expected, "mismatch for {:?}", cat);
        }
    }

    // -- OutputType --

    #[test]
    fn test_output_type_default_is_four_severity_levels() {
        assert_eq!(OutputType::default(), OutputType::FourSeverityLevels);
    }

    #[test]
    fn test_output_type_serializes_correctly() {
        assert_eq!(
            serde_json::to_string(&OutputType::FourSeverityLevels).unwrap(),
            "\"FourSeverityLevels\""
        );
        assert_eq!(
            serde_json::to_string(&OutputType::EightSeverityLevels).unwrap(),
            "\"EightSeverityLevels\""
        );
    }

    // -- ImageOutputType --

    #[test]
    fn test_image_output_type_only_four_severity_levels() {
        assert_eq!(
            ImageOutputType::default(),
            ImageOutputType::FourSeverityLevels
        );
        assert_eq!(
            serde_json::to_string(&ImageOutputType::FourSeverityLevels).unwrap(),
            "\"FourSeverityLevels\""
        );
    }

    // -- API version constant --

    #[test]
    fn test_api_version_constant_has_correct_format() {
        assert!(
            CONTENT_SAFETY_API_VERSION.starts_with("api-version="),
            "constant must start with 'api-version='"
        );
        let version = CONTENT_SAFETY_API_VERSION
            .strip_prefix("api-version=")
            .unwrap();
        assert_eq!(version.len(), 10, "version must be YYYY-MM-DD format");
        assert_eq!(version, "2024-09-01");
    }

    #[test]
    fn test_limit_constants() {
        assert_eq!(MAX_TEXT_LENGTH, 10_000);
        assert_eq!(MAX_BLOCKLIST_NAME_LENGTH, 64);
        assert_eq!(MAX_DESCRIPTION_LENGTH, 1_024);
        assert_eq!(MAX_ITEM_TEXT_LENGTH, 128);
    }

    // -- CategoryAnalysis --

    #[test]
    fn test_category_analysis_deserialization() {
        let json = r#"{"category": "Hate", "severity": 4}"#;
        let analysis: CategoryAnalysis = serde_json::from_str(json).unwrap();
        assert_eq!(analysis.category, HarmCategory::Hate);
        assert_eq!(analysis.severity, 4);
    }
}
