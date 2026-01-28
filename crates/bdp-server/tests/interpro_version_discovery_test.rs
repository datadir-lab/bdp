//! Tests for InterPro version discovery
//!
//! Note: Most tests are unit tests in the version_discovery module.
//! These are integration tests that verify the overall structure.

use bdp_server::ingest::interpro::version_discovery::DiscoveredVersion;

#[test]
fn test_discovered_version_ordering() {
    let v1 = DiscoveredVersion {
        external_version: "96.0".to_string(),
        major: 96,
        minor: 0,
        release_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        is_current: false,
        ftp_directory: "96.0".to_string(),
    };

    let v2 = DiscoveredVersion {
        external_version: "97.0".to_string(),
        major: 97,
        minor: 0,
        release_date: chrono::NaiveDate::from_ymd_opt(2024, 4, 1).unwrap(),
        is_current: false,
        ftp_directory: "97.0".to_string(),
    };

    assert!(v1 < v2);
}

#[test]
fn test_version_parsing() {
    let (major, minor) = DiscoveredVersion::parse_version("96.0").unwrap();
    assert_eq!(major, 96);
    assert_eq!(minor, 0);

    let (major, minor) = DiscoveredVersion::parse_version("100.0").unwrap();
    assert_eq!(major, 100);
    assert_eq!(minor, 0);
}

#[test]
fn test_version_parsing_invalid() {
    assert!(DiscoveredVersion::parse_version("96").is_err());
    assert!(DiscoveredVersion::parse_version("96.0.1").is_err());
    assert!(DiscoveredVersion::parse_version("invalid").is_err());
}

#[test]
fn test_estimate_release_date() {
    let date = DiscoveredVersion::estimate_release_date(96, 0);

    // Should be a valid date
    assert!(date.year() >= 2001);
    assert!(date.year() <= 2100);
    assert!(date.month() >= 1);
    assert!(date.month() <= 12);
}
