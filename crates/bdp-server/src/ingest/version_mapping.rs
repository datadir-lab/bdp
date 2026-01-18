//! Version mapping utilities
//!
//! Utilities for mapping between internal and external version identifiers.
//! Currently uses a simple mapping strategy where internal version is always "1.0"
//! and external versions are preserved as-is (e.g., "2024_01", "2025_06").

/// Version mapper for converting between internal and external version formats
///
/// Current strategy:
/// - Internal version: Always "1.0" (our schema version)
/// - External version: Preserved as-is from source (e.g., "2024_01" for UniProt)
///
/// This simple approach allows us to version our data schema independently
/// from the upstream source versioning.
pub struct VersionMapper;

impl VersionMapper {
    /// Create a new version mapper
    pub fn new() -> Self {
        Self
    }

    /// Convert external version to internal format
    ///
    /// Currently returns "1.0" for all external versions as we're using
    /// a single internal schema version.
    ///
    /// # Arguments
    /// * `external_version` - External version string (e.g., "2024_01")
    ///
    /// # Returns
    /// Internal version string (currently always "1.0")
    pub fn external_to_internal(&self, _external_version: &str) -> anyhow::Result<String> {
        Ok("1.0".to_string())
    }

    /// Convert internal version to external format
    ///
    /// This is the inverse operation, but since our internal version is generic ("1.0"),
    /// we cannot reliably reconstruct the external version from it alone.
    /// This method is provided for API completeness but should be used with caution.
    ///
    /// # Arguments
    /// * `internal_version` - Internal version string (expected to be "1.0")
    ///
    /// # Returns
    /// Error indicating this operation is not supported in the current implementation
    pub fn internal_to_external(&self, internal_version: &str) -> anyhow::Result<String> {
        // Since internal version is always "1.0" and doesn't encode external version info,
        // we cannot convert back without additional context
        anyhow::bail!(
            "Cannot convert internal version '{}' to external format without additional context. \
             Internal version '1.0' represents our schema version, not the source version.",
            internal_version
        )
    }
}

impl Default for VersionMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_mapper_new() {
        let _mapper = VersionMapper::new();
    }
}
