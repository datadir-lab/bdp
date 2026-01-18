pub mod data_sources;
pub mod files;
pub mod jobs;
pub mod organisms;
pub mod organizations;
pub mod protein_metadata;
pub mod resolve;
pub mod search;
pub mod version_files;

use axum::Router;
use crate::storage::Storage;

#[derive(Clone)]
pub struct FeatureState {
    pub db: sqlx::PgPool,
    pub storage: Storage,
}

pub fn router(state: FeatureState) -> Router<()> {
    Router::new()
        .nest("/organizations", organizations::organizations_routes().with_state(state.db.clone()))
        .nest("/data-sources", data_sources::data_sources_routes().with_state(state.db.clone()))
        .nest("/search", search::search_routes().with_state(state.db.clone()))
        .nest("/resolve", resolve::resolve_routes().with_state(state.db.clone()))
        .nest("/jobs", jobs::jobs_routes().with_state(state.db.clone()))
        .nest("/files", files::files_routes().with_state(state.storage.clone()))
}
