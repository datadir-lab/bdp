//! Shared validation utilities
//!
//! Provides common validation functions for input data across commands and queries.
//!
//! # Examples
//!
//! ```rust,ignore
//! use bdp_server::features::shared::validation::{validate_slug, validate_name, validate_url};
//!
//! // Validate a slug
//! validate_slug("my-project", 100)?;
//!
//! // Validate a name
//! validate_name("My Project", 256)?;
//!
//! // Validate a URL
//! if let Some(url) = &website {
//!     validate_url(url, "website")?;
//! }
//! ```

use thiserror::Error;

/// Errors that can occur during slug validation
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SlugValidationError {
    #[error("Slug is required and cannot be empty")]
    Required,

    #[error("Slug must be between 1 and {max_length} characters")]
    TooLong { max_length: usize },

    #[error("Slug can only contain lowercase letters, numbers, and hyphens")]
    InvalidFormat,

    #[error("Slug cannot start or end with a hyphen")]
    InvalidHyphenPlacement,
}

/// Errors that can occur during name validation
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NameValidationError {
    #[error("Name is required and cannot be empty")]
    Required,

    #[error("Name must be between 1 and {max_length} characters")]
    TooLong { max_length: usize },
}

/// Errors that can occur during URL validation
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum UrlValidationError {
    #[error("{field_name} URL is invalid: must start with http:// or https://")]
    InvalidFormat { field_name: String },
}

/// Validate a slug (URL-safe identifier)
///
/// # Rules
/// - Must not be empty
/// - Must not exceed max_length characters
/// - Must contain only lowercase letters, numbers, and hyphens
/// - Must not start or end with a hyphen
///
/// # Arguments
/// * `slug` - The slug to validate
/// * `max_length` - Maximum allowed length (typically 100 or 255)
///
/// # Returns
/// Ok(()) if valid, or a SlugValidationError
pub fn validate_slug(slug: &str, max_length: usize) -> Result<(), SlugValidationError> {
    if slug.is_empty() {
        return Err(SlugValidationError::Required);
    }

    if slug.len() > max_length {
        return Err(SlugValidationError::TooLong { max_length });
    }

    // Check for valid characters
    if !slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(SlugValidationError::InvalidFormat);
    }

    // Check hyphen placement
    if slug.starts_with('-') || slug.ends_with('-') {
        return Err(SlugValidationError::InvalidHyphenPlacement);
    }

    Ok(())
}

/// Validate a name field
///
/// # Rules
/// - Must not be empty (after trimming whitespace)
/// - Must not exceed max_length characters
///
/// # Arguments
/// * `name` - The name to validate
/// * `max_length` - Maximum allowed length (typically 256)
///
/// # Returns
/// Ok(()) if valid, or a NameValidationError
pub fn validate_name(name: &str, max_length: usize) -> Result<(), NameValidationError> {
    if name.trim().is_empty() {
        return Err(NameValidationError::Required);
    }

    if name.len() > max_length {
        return Err(NameValidationError::TooLong { max_length });
    }

    Ok(())
}

/// Validate a URL field
///
/// # Rules
/// - Must start with http:// or https://
/// - Empty strings are considered valid (use Option<String> and check for Some)
///
/// # Arguments
/// * `url` - The URL to validate
/// * `field_name` - Name of the field (for error messages)
///
/// # Returns
/// Ok(()) if valid, or a UrlValidationError
pub fn validate_url(url: &str, field_name: &str) -> Result<(), UrlValidationError> {
    if url.is_empty() {
        return Ok(());
    }

    if !is_valid_url(url) {
        return Err(UrlValidationError::InvalidFormat {
            field_name: field_name.to_string(),
        });
    }

    Ok(())
}

/// Check if a URL is valid (starts with http:// or https://)
///
/// This is a basic validation. For more thorough validation, consider using
/// a dedicated URL parsing library.
#[inline]
pub fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

/// Validate an optional URL field
///
/// Convenience function that handles Option<String> directly.
///
/// # Arguments
/// * `url` - Optional URL to validate
/// * `field_name` - Name of the field (for error messages)
///
/// # Returns
/// Ok(()) if valid or None, or a UrlValidationError
pub fn validate_optional_url(
    url: Option<&str>,
    field_name: &str,
) -> Result<(), UrlValidationError> {
    if let Some(url) = url {
        validate_url(url, field_name)?;
    }
    Ok(())
}

/// Common source type values and validation
pub const VALID_SOURCE_TYPES: &[&str] = &[
    "protein",
    "genome",
    "organism",
    "taxonomy",
    "bundle",
    "transcript",
    "annotation",
    "structure",
    "pathway",
    "other",
];

/// Validate a source type value
///
/// # Arguments
/// * `source_type` - The source type to validate
///
/// # Returns
/// Ok(()) if valid, or an error message
pub fn validate_source_type(source_type: &str) -> Result<(), String> {
    if source_type.is_empty() {
        return Err("Source type is required".to_string());
    }

    if !VALID_SOURCE_TYPES.contains(&source_type) {
        return Err(format!(
            "Invalid source type: {}. Must be one of: {}",
            source_type,
            VALID_SOURCE_TYPES.join(", ")
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Slug validation tests
    #[test]
    fn test_validate_slug_valid() {
        assert!(validate_slug("valid-slug", 100).is_ok());
        assert!(validate_slug("my-project-123", 100).is_ok());
        assert!(validate_slug("a", 100).is_ok());
        assert!(validate_slug("123", 100).is_ok());
    }

    #[test]
    fn test_validate_slug_empty() {
        assert_eq!(validate_slug("", 100), Err(SlugValidationError::Required));
    }

    #[test]
    fn test_validate_slug_too_long() {
        let long_slug = "a".repeat(101);
        assert_eq!(
            validate_slug(&long_slug, 100),
            Err(SlugValidationError::TooLong { max_length: 100 })
        );
    }

    #[test]
    fn test_validate_slug_invalid_chars() {
        assert_eq!(validate_slug("UPPERCASE", 100), Err(SlugValidationError::InvalidFormat));
        assert_eq!(validate_slug("has spaces", 100), Err(SlugValidationError::InvalidFormat));
        assert_eq!(validate_slug("has_underscore", 100), Err(SlugValidationError::InvalidFormat));
        assert_eq!(validate_slug("has@special", 100), Err(SlugValidationError::InvalidFormat));
    }

    #[test]
    fn test_validate_slug_hyphen_placement() {
        assert_eq!(
            validate_slug("-starts-with-hyphen", 100),
            Err(SlugValidationError::InvalidHyphenPlacement)
        );
        assert_eq!(
            validate_slug("ends-with-hyphen-", 100),
            Err(SlugValidationError::InvalidHyphenPlacement)
        );
    }

    // Name validation tests
    #[test]
    fn test_validate_name_valid() {
        assert!(validate_name("Valid Name", 256).is_ok());
        assert!(validate_name("a", 256).is_ok());
    }

    #[test]
    fn test_validate_name_empty() {
        assert_eq!(validate_name("", 256), Err(NameValidationError::Required));
        assert_eq!(validate_name("   ", 256), Err(NameValidationError::Required));
    }

    #[test]
    fn test_validate_name_too_long() {
        let long_name = "a".repeat(257);
        assert_eq!(
            validate_name(&long_name, 256),
            Err(NameValidationError::TooLong { max_length: 256 })
        );
    }

    // URL validation tests
    #[test]
    fn test_validate_url_valid() {
        assert!(validate_url("https://example.com", "website").is_ok());
        assert!(validate_url("http://example.com", "website").is_ok());
        assert!(validate_url("https://example.com/path?query=1", "website").is_ok());
        assert!(validate_url("", "website").is_ok()); // Empty is valid
    }

    #[test]
    fn test_validate_url_invalid() {
        assert!(validate_url("ftp://example.com", "website").is_err());
        assert!(validate_url("example.com", "website").is_err());
        assert!(validate_url("not a url", "website").is_err());
    }

    #[test]
    fn test_validate_optional_url() {
        assert!(validate_optional_url(None, "website").is_ok());
        assert!(validate_optional_url(Some("https://example.com"), "website").is_ok());
        assert!(validate_optional_url(Some("invalid"), "website").is_err());
    }

    #[test]
    fn test_is_valid_url() {
        assert!(is_valid_url("https://example.com"));
        assert!(is_valid_url("http://example.com"));
        assert!(!is_valid_url("ftp://example.com"));
        assert!(!is_valid_url("example.com"));
    }

    // Source type validation tests
    #[test]
    fn test_validate_source_type_valid() {
        assert!(validate_source_type("protein").is_ok());
        assert!(validate_source_type("genome").is_ok());
        assert!(validate_source_type("organism").is_ok());
    }

    #[test]
    fn test_validate_source_type_invalid() {
        assert!(validate_source_type("").is_err());
        assert!(validate_source_type("invalid").is_err());
    }
}
