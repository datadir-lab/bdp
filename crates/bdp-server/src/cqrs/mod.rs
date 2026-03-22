pub use mediator::DefaultAsyncMediator;
use sqlx::PgPool;

use crate::storage::Storage;

pub mod middleware;

pub type AppMediator = DefaultAsyncMediator;

pub fn build_mediator(pool: PgPool, storage: Storage) -> AppMediator {
    DefaultAsyncMediator::builder()
        // ================================================================
        // Organizations
        // ================================================================
        .add_handler({
            let pool = pool.clone();
            move |cmd| {
                let pool = pool.clone();
                async move { crate::features::organizations::commands::create::handle(pool, cmd).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |cmd| {
                let pool = pool.clone();
                async move { crate::features::organizations::commands::update::handle(pool, cmd).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |cmd| {
                let pool = pool.clone();
                async move { crate::features::organizations::commands::delete::handle(pool, cmd).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::organizations::queries::list::handle(pool, query).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::organizations::queries::get::handle(pool, query).await }
            }
        })
        // ================================================================
        // Data Sources
        // ================================================================
        .add_handler({
            let pool = pool.clone();
            move |cmd| {
                let pool = pool.clone();
                async move { crate::features::data_sources::commands::create::handle(pool, cmd).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |cmd| {
                let pool = pool.clone();
                async move { crate::features::data_sources::commands::update::handle(pool, cmd).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |cmd| {
                let pool = pool.clone();
                async move { crate::features::data_sources::commands::delete::handle(pool, cmd).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |cmd| {
                let pool = pool.clone();
                async move { crate::features::data_sources::commands::publish::handle(pool, cmd).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::data_sources::queries::list::handle(pool, query).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::data_sources::queries::get::handle(pool, query).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::data_sources::queries::get_version::handle(pool, query).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::data_sources::queries::list_dependencies::handle(pool, query).await }
            }
        })
        // ================================================================
        // Search
        // ================================================================
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::search::queries::unified_search::handle(pool, query).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::search::queries::suggestions::handle(pool, query).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |cmd| {
                let pool = pool.clone();
                async move { crate::features::search::queries::refresh_search_index::handle(pool, cmd).await }
            }
        })
        // ================================================================
        // Resolve
        // ================================================================
        .add_handler({
            let pool = pool.clone();
            let storage = storage.clone();
            move |query| {
                let pool = pool.clone();
                let storage = storage.clone();
                async move { crate::features::resolve::queries::resolve_manifest::handle(pool, storage, query).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |cmd| {
                let pool = pool.clone();
                async move { crate::features::resolve::commands::record_download::handle(pool, cmd).await }
            }
        })
        // ================================================================
        // Jobs
        // ================================================================
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::jobs::queries::list_jobs::handle(pool, query).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::jobs::queries::get_job::handle(pool, query).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::jobs::queries::get_sync_status::handle_list(pool, query).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::jobs::queries::get_sync_status::handle_get(pool, query).await }
            }
        })
        // ================================================================
        // Files (storage-only handlers)
        // ================================================================
        .add_handler({
            let storage = storage.clone();
            move |cmd| {
                let storage = storage.clone();
                async move { crate::features::files::commands::upload::handle(storage, cmd).await }
            }
        })
        .add_handler({
            let storage = storage.clone();
            move |query| {
                let storage = storage.clone();
                async move { crate::features::files::queries::download::handle(storage, query).await }
            }
        })
        // ================================================================
        // Query (SQL execution)
        // ================================================================
        .add_handler({
            let pool = pool.clone();
            move |req| {
                let pool = pool.clone();
                async move { crate::features::query::queries::execute_query::handle(pool, req).await }
            }
        })
        // ================================================================
        // Protein Metadata Query (data_sources sub-feature)
        // ================================================================
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::data_sources::queries::get_protein_metadata::handle(pool, query).await }
            }
        })
        // ================================================================
        // Organisms
        // ================================================================
        .add_handler({
            let pool = pool.clone();
            move |cmd| {
                let pool = pool.clone();
                async move { crate::features::organisms::commands::create::handle(pool, cmd).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::organisms::queries::get_or_create::handle(pool, query).await }
            }
        })
        // ================================================================
        // Version Files
        // ================================================================
        .add_handler({
            let pool = pool.clone();
            move |cmd| {
                let pool = pool.clone();
                async move { crate::features::version_files::commands::add_batch::handle(pool, cmd).await }
            }
        })
        // ================================================================
        // Protein Metadata
        // ================================================================
        .add_handler({
            let pool = pool.clone();
            move |cmd| {
                let pool = pool.clone();
                async move { crate::features::protein_metadata::commands::insert::handle(pool, cmd).await }
            }
        })
        // ================================================================
        // Vectors
        // ================================================================
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::vectors::queries::get_stats::handle(pool, query).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::vectors::queries::semantic_search::handle(pool, query).await }
            }
        })
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::vectors::queries::get_neighbors::handle(pool, query).await }
            }
        })
        .add_handler({
            let storage = storage.clone();
            move |query| {
                let storage = storage.clone();
                async move { crate::features::vectors::queries::get_tile::handle(storage, query).await }
            }
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_mediator_builds() {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost".to_string());

        if let Ok(pool) = PgPool::connect(&database_url).await {
            let storage_config = crate::storage::config::StorageConfig::for_minio(
                "http://127.0.0.1:19999",
                "bdp-test",
            );
            let storage = crate::storage::Storage::new(storage_config)
                .await
                .expect("Failed to create test storage");
            let _mediator = build_mediator(pool, storage);
        }
    }
}
