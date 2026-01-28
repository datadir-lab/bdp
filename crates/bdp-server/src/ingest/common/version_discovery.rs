//! Generic version discovery trait and utilities
//!
//! Provides a common interface for version discovery across different data sources,
//! eliminating code duplication and standardizing version handling.

use chrono::NaiveDate;
use std::cmp::Ordering;

/// Common trait for discovered data source versions
///
/// All data sources implement this trait to provide a consistent interface
/// for version discovery, sorting, and tracking.
///
/// # Example
///
/// ```rust,ignore
/// use chrono::NaiveDate;
/// use crate::ingest::common::version_discovery::DiscoveredVersion;
///
/// #[derive(Debug, Clone, PartialEq, Eq)]
/// struct MyVersion {
///     external_version: String,
///     release_date: NaiveDate,
///     release_url: String,
/// }
///
/// impl DiscoveredVersion for MyVersion {
///     fn external_version(&self) -> &str {
///         &self.external_version
///     }
///
///     fn release_date(&self) -> NaiveDate {
///         self.release_date
///     }
///
///     fn release_url(&self) -> Option<&str> {
///         Some(&self.release_url)
///     }
/// }
/// ```
pub trait DiscoveredVersion: Clone + PartialEq + Eq {
    /// Get the external version identifier (e.g., "2024_01", "2025-01-01", "GB_Release_257.0")
    fn external_version(&self) -> &str;

    /// Get the release date for this version
    fn release_date(&self) -> NaiveDate;

    /// Get the release URL if available (optional)
    fn release_url(&self) -> Option<&str> {
        None
    }

    /// Compare versions by date, then by external version string
    ///
    /// This provides a standard ordering: oldest to newest by date,
    /// then alphabetically by version string for same-date releases.
    fn compare_versions(&self, other: &Self) -> Ordering {
        match self.release_date().cmp(&other.release_date()) {
            Ordering::Equal => self.external_version().cmp(other.external_version()),
            other => other,
        }
    }
}

/// Helper macro to implement Ord and PartialOrd using the DiscoveredVersion trait
///
/// This eliminates boilerplate for implementing ordering on version types.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Debug, Clone, PartialEq, Eq)]
/// struct MyVersion {
///     external_version: String,
///     release_date: NaiveDate,
/// }
///
/// impl DiscoveredVersion for MyVersion {
///     fn external_version(&self) -> &str { &self.external_version }
///     fn release_date(&self) -> NaiveDate { self.release_date }
/// }
///
/// impl_version_ordering!(MyVersion);
/// ```
#[macro_export]
macro_rules! impl_version_ordering {
    ($type:ty) => {
        impl PartialOrd for $type {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $type {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                $crate::ingest::common::version_discovery::DiscoveredVersion::compare_versions(
                    self, other,
                )
            }
        }
    };
}

/// Generic version filtering utilities
pub struct VersionFilter;

impl VersionFilter {
    /// Filter versions that are not in the ingested list
    pub fn filter_new_versions<T: DiscoveredVersion>(
        discovered: Vec<T>,
        ingested_versions: &[String],
    ) -> Vec<T> {
        discovered
            .into_iter()
            .filter(|v| !ingested_versions.contains(&v.external_version().to_string()))
            .collect()
    }

    /// Filter versions by date range (inclusive)
    pub fn filter_by_date_range<T: DiscoveredVersion>(
        versions: Vec<T>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Vec<T> {
        versions
            .into_iter()
            .filter(|v| {
                let date = v.release_date();
                let after_start = start_date.map_or(true, |start| date >= start);
                let before_end = end_date.map_or(true, |end| date <= end);
                after_start && before_end
            })
            .collect()
    }

    /// Sort versions by date (oldest first)
    pub fn sort_versions<T: DiscoveredVersion>(mut versions: Vec<T>) -> Vec<T> {
        versions.sort_by(|a, b| a.compare_versions(b));
        versions
    }

    /// Get the newest version from a list
    pub fn get_newest<T: DiscoveredVersion>(versions: &[T]) -> Option<&T> {
        versions.iter().max_by(|a, b| a.compare_versions(b))
    }

    /// Get the oldest version from a list
    pub fn get_oldest<T: DiscoveredVersion>(versions: &[T]) -> Option<&T> {
        versions.iter().min_by(|a, b| a.compare_versions(b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestVersion {
        external_version: String,
        release_date: NaiveDate,
        release_url: Option<String>,
    }

    impl DiscoveredVersion for TestVersion {
        fn external_version(&self) -> &str {
            &self.external_version
        }

        fn release_date(&self) -> NaiveDate {
            self.release_date
        }

        fn release_url(&self) -> Option<&str> {
            self.release_url.as_deref()
        }
    }

    impl_version_ordering!(TestVersion);

    fn create_test_version(version: &str, year: i32, month: u32, day: u32) -> TestVersion {
        TestVersion {
            external_version: version.to_string(),
            release_date: NaiveDate::from_ymd_opt(year, month, day).unwrap(),
            release_url: Some(format!("https://example.com/{}", version)),
        }
    }

    #[test]
    fn test_version_ordering() {
        let v1 = create_test_version("2024-11-01", 2024, 11, 1);
        let v2 = create_test_version("2024-12-01", 2024, 12, 1);
        let v3 = create_test_version("2025-01-01", 2025, 1, 1);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
    }

    #[test]
    fn test_version_ordering_same_date() {
        let v1 = create_test_version("alpha", 2024, 11, 1);
        let v2 = create_test_version("beta", 2024, 11, 1);

        assert!(v1 < v2); // Alphabetical order for same date
    }

    #[test]
    fn test_version_sorting() {
        let mut versions = vec![
            create_test_version("2025-01-01", 2025, 1, 1),
            create_test_version("2024-11-01", 2024, 11, 1),
            create_test_version("2024-12-01", 2024, 12, 1),
        ];

        versions.sort();

        assert_eq!(versions[0].external_version(), "2024-11-01");
        assert_eq!(versions[1].external_version(), "2024-12-01");
        assert_eq!(versions[2].external_version(), "2025-01-01");
    }

    #[test]
    fn test_filter_new_versions() {
        let discovered = vec![
            create_test_version("2024-11-01", 2024, 11, 1),
            create_test_version("2024-12-01", 2024, 12, 1),
            create_test_version("2025-01-01", 2025, 1, 1),
        ];

        let ingested = vec!["2024-11-01".to_string()];

        let new_versions = VersionFilter::filter_new_versions(discovered, &ingested);

        assert_eq!(new_versions.len(), 2);
        assert_eq!(new_versions[0].external_version(), "2024-12-01");
        assert_eq!(new_versions[1].external_version(), "2025-01-01");
    }

    #[test]
    fn test_filter_by_date_range() {
        let versions = vec![
            create_test_version("2024-11-01", 2024, 11, 1),
            create_test_version("2024-12-01", 2024, 12, 1),
            create_test_version("2025-01-01", 2025, 1, 1),
            create_test_version("2025-02-01", 2025, 2, 1),
        ];

        let start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();

        let filtered = VersionFilter::filter_by_date_range(versions, Some(start), Some(end));

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].external_version(), "2024-12-01");
        assert_eq!(filtered[1].external_version(), "2025-01-01");
    }

    #[test]
    fn test_filter_by_date_range_start_only() {
        let versions = vec![
            create_test_version("2024-11-01", 2024, 11, 1),
            create_test_version("2024-12-01", 2024, 12, 1),
            create_test_version("2025-01-01", 2025, 1, 1),
        ];

        let start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();

        let filtered = VersionFilter::filter_by_date_range(versions, Some(start), None);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].external_version(), "2024-12-01");
        assert_eq!(filtered[1].external_version(), "2025-01-01");
    }

    #[test]
    fn test_sort_versions() {
        let versions = vec![
            create_test_version("2025-01-01", 2025, 1, 1),
            create_test_version("2024-11-01", 2024, 11, 1),
            create_test_version("2024-12-01", 2024, 12, 1),
        ];

        let sorted = VersionFilter::sort_versions(versions);

        assert_eq!(sorted[0].external_version(), "2024-11-01");
        assert_eq!(sorted[1].external_version(), "2024-12-01");
        assert_eq!(sorted[2].external_version(), "2025-01-01");
    }

    #[test]
    fn test_get_newest() {
        let versions = vec![
            create_test_version("2024-11-01", 2024, 11, 1),
            create_test_version("2024-12-01", 2024, 12, 1),
            create_test_version("2025-01-01", 2025, 1, 1),
        ];

        let newest = VersionFilter::get_newest(&versions).unwrap();
        assert_eq!(newest.external_version(), "2025-01-01");
    }

    #[test]
    fn test_get_oldest() {
        let versions = vec![
            create_test_version("2024-12-01", 2024, 12, 1),
            create_test_version("2024-11-01", 2024, 11, 1),
            create_test_version("2025-01-01", 2025, 1, 1),
        ];

        let oldest = VersionFilter::get_oldest(&versions).unwrap();
        assert_eq!(oldest.external_version(), "2024-11-01");
    }

    #[test]
    fn test_get_newest_empty() {
        let versions: Vec<TestVersion> = vec![];
        assert!(VersionFilter::get_newest(&versions).is_none());
    }

    #[test]
    fn test_get_oldest_empty() {
        let versions: Vec<TestVersion> = vec![];
        assert!(VersionFilter::get_oldest(&versions).is_none());
    }
}
