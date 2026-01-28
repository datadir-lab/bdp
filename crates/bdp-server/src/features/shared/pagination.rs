//! Shared pagination utilities
//!
//! Provides common pagination types and helpers used across list queries.
//!
//! # Examples
//!
//! ```rust,ignore
//! use bdp_server::features::shared::pagination::{PaginationParams, PaginationMetadata, Paginated};
//!
//! let params = PaginationParams::new(Some(2), Some(20));
//! let offset = params.offset();
//!
//! // After fetching data...
//! let metadata = PaginationMetadata::new(params.page(), params.per_page(), 100);
//! ```

use serde::{Deserialize, Serialize};

/// Common pagination request parameters
///
/// Used in list queries to specify page and items per page.
/// Provides sensible defaults (page 1, 20 items per page).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PaginationParams {
    /// Page number (1-indexed). Defaults to 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i64>,

    /// Items per page. Defaults to 20, clamped to 1-100.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<i64>,
}

impl PaginationParams {
    /// Create new pagination parameters
    pub fn new(page: Option<i64>, per_page: Option<i64>) -> Self {
        Self { page, per_page }
    }

    /// Get the page number (1-indexed), defaulting to 1
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    /// Get items per page, defaulting to 20 and clamped to 1-100
    pub fn per_page(&self) -> i64 {
        self.per_page.unwrap_or(20).clamp(1, 100)
    }

    /// Calculate the offset for SQL OFFSET clause
    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.per_page()
    }

    /// Validate pagination parameters
    ///
    /// Returns an error message if validation fails.
    pub fn validate(&self) -> Result<(), &'static str> {
        if let Some(page) = self.page {
            if page < 1 {
                return Err("Page must be greater than 0");
            }
        }
        if let Some(per_page) = self.per_page {
            if per_page < 1 || per_page > 100 {
                return Err("Per page must be between 1 and 100");
            }
        }
        Ok(())
    }
}

/// Pagination metadata for response
///
/// Contains information about the current page and total results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMetadata {
    /// Current page number (1-indexed)
    pub page: i64,

    /// Items per page
    pub per_page: i64,

    /// Total number of items
    pub total: i64,

    /// Total number of pages
    pub pages: i64,

    /// Whether there is a next page
    pub has_next: bool,

    /// Whether there is a previous page
    pub has_prev: bool,
}

impl PaginationMetadata {
    /// Create new pagination metadata from query results
    pub fn new(page: i64, per_page: i64, total: i64) -> Self {
        let pages = if total == 0 {
            0
        } else {
            ((total as f64) / (per_page as f64)).ceil() as i64
        };

        Self {
            page,
            per_page,
            total,
            pages,
            has_next: page < pages,
            has_prev: page > 1,
        }
    }

    /// Create pagination metadata from params and total count
    pub fn from_params(params: &PaginationParams, total: i64) -> Self {
        Self::new(params.page(), params.per_page(), total)
    }
}

/// Wrapper for paginated list responses
///
/// Generic container for paginated results of any item type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paginated<T> {
    /// List of items for the current page
    pub items: Vec<T>,

    /// Pagination metadata
    pub pagination: PaginationMetadata,
}

impl<T> Paginated<T> {
    /// Create a new paginated response
    pub fn new(items: Vec<T>, pagination: PaginationMetadata) -> Self {
        Self { items, pagination }
    }

    /// Create a paginated response from items, params, and total count
    pub fn from_items(items: Vec<T>, params: &PaginationParams, total: i64) -> Self {
        Self {
            items,
            pagination: PaginationMetadata::from_params(params, total),
        }
    }

    /// Map items to a different type
    pub fn map<U, F: FnMut(T) -> U>(self, f: F) -> Paginated<U> {
        Paginated {
            items: self.items.into_iter().map(f).collect(),
            pagination: self.pagination,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_params_defaults() {
        let params = PaginationParams::default();
        assert_eq!(params.page(), 1);
        assert_eq!(params.per_page(), 20);
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn test_pagination_params_custom() {
        let params = PaginationParams::new(Some(3), Some(50));
        assert_eq!(params.page(), 3);
        assert_eq!(params.per_page(), 50);
        assert_eq!(params.offset(), 100);
    }

    #[test]
    fn test_pagination_params_clamping() {
        let params = PaginationParams::new(Some(-1), Some(200));
        assert_eq!(params.page(), 1);
        assert_eq!(params.per_page(), 100);
    }

    #[test]
    fn test_pagination_params_validation() {
        let valid = PaginationParams::new(Some(1), Some(50));
        assert!(valid.validate().is_ok());

        let invalid_page = PaginationParams::new(Some(0), Some(20));
        assert_eq!(invalid_page.validate(), Err("Page must be greater than 0"));

        let invalid_per_page = PaginationParams::new(Some(1), Some(101));
        assert_eq!(
            invalid_per_page.validate(),
            Err("Per page must be between 1 and 100")
        );
    }

    #[test]
    fn test_pagination_metadata() {
        let meta = PaginationMetadata::new(2, 10, 25);
        assert_eq!(meta.page, 2);
        assert_eq!(meta.per_page, 10);
        assert_eq!(meta.total, 25);
        assert_eq!(meta.pages, 3);
        assert!(meta.has_prev);
        assert!(meta.has_next);
    }

    #[test]
    fn test_pagination_metadata_empty() {
        let meta = PaginationMetadata::new(1, 10, 0);
        assert_eq!(meta.pages, 0);
        assert!(!meta.has_prev);
        assert!(!meta.has_next);
    }

    #[test]
    fn test_pagination_metadata_last_page() {
        let meta = PaginationMetadata::new(3, 10, 25);
        assert!(meta.has_prev);
        assert!(!meta.has_next);
    }

    #[test]
    fn test_paginated_map() {
        let paginated = Paginated::new(
            vec![1, 2, 3],
            PaginationMetadata::new(1, 10, 3),
        );

        let mapped = paginated.map(|x| x * 2);
        assert_eq!(mapped.items, vec![2, 4, 6]);
        assert_eq!(mapped.pagination.total, 3);
    }
}
