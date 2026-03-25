// crates/bdp-ingest/src/common/batch.rs

/// Configuration for chunked batch inserts.
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum number of rows per INSERT statement.
    pub chunk_size: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self { chunk_size: 500 }
    }
}

impl BatchConfig {
    pub fn new(chunk_size: usize) -> Self {
        Self { chunk_size }
    }
}

/// Split a slice into chunks of `chunk_size` for batch processing.
pub fn chunks<T>(items: &[T], chunk_size: usize) -> impl Iterator<Item = &[T]> {
    items.chunks(chunk_size)
}
