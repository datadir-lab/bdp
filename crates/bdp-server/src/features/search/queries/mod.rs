pub mod refresh_search_index;
pub mod suggestions;
pub mod unified_search;

pub use refresh_search_index::{
    RefreshSearchIndexCommand, RefreshSearchIndexError, RefreshSearchIndexResponse,
};

pub use suggestions::{
    SearchSuggestionItem, SearchSuggestionsError, SearchSuggestionsQuery,
    SearchSuggestionsResponse,
};

pub use unified_search::{
    OrganismInfo, SearchResultItem, UnifiedSearchError, UnifiedSearchQuery,
    UnifiedSearchResponse,
};
// Re-export from shared module to avoid privacy issues
pub use crate::features::shared::pagination::PaginationMetadata;
