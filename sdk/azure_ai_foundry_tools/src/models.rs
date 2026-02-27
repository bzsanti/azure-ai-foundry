//! Shared types for Azure AI Foundry Vision and Document Intelligence services.
//!
//! This module contains common types and constants used across the tools crate.

use serde::Deserialize;

/// API version query parameter for Vision Image Analysis 4.0 requests.
pub(crate) const VISION_API_VERSION: &str = "api-version=2024-02-01";

/// API version query parameter for Document Intelligence v4.0 requests.
pub(crate) const DOCUMENT_INTELLIGENCE_API_VERSION: &str = "api-version=2024-11-30";

/// A bounding box in pixel coordinates.
#[derive(Debug, Clone, Deserialize)]
pub struct BoundingBox {
    /// X-coordinate of the top-left corner.
    pub x: i32,
    /// Y-coordinate of the top-left corner.
    pub y: i32,
    /// Width of the bounding box in pixels.
    pub w: i32,
    /// Height of the bounding box in pixels.
    pub h: i32,
}

/// Metadata about the analyzed image dimensions.
#[derive(Debug, Clone, Deserialize)]
pub struct ImageMetadata {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

/// A point in image coordinates.
#[derive(Debug, Clone, Deserialize)]
pub struct ImagePoint {
    /// X-coordinate.
    pub x: i32,
    /// Y-coordinate.
    pub y: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vision_api_version_format() {
        assert_eq!(VISION_API_VERSION, "api-version=2024-02-01");
    }

    #[test]
    fn test_document_intelligence_api_version_format() {
        assert_eq!(DOCUMENT_INTELLIGENCE_API_VERSION, "api-version=2024-11-30");
    }

    #[test]
    fn test_bounding_box_deserialization() {
        let json = r#"{"x": 10, "y": 20, "w": 100, "h": 50}"#;
        let bbox: BoundingBox = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(bbox.x, 10);
        assert_eq!(bbox.y, 20);
        assert_eq!(bbox.w, 100);
        assert_eq!(bbox.h, 50);
    }

    #[test]
    fn test_image_metadata_deserialization() {
        let json = r#"{"width": 1920, "height": 1080}"#;
        let metadata: ImageMetadata = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(metadata.width, 1920);
        assert_eq!(metadata.height, 1080);
    }

    #[test]
    fn test_image_point_deserialization() {
        let json = r#"{"x": 42, "y": 99}"#;
        let point: ImagePoint = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(point.x, 42);
        assert_eq!(point.y, 99);
    }
}
