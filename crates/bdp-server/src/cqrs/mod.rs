pub use mediator::DefaultAsyncMediator;
use sqlx::PgPool;

pub mod middleware;

pub type AppMediator = DefaultAsyncMediator;

pub fn build_mediator(pool: PgPool) -> AppMediator {
    DefaultAsyncMediator::builder()
        // Organizations
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
        // Data Sources
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
        // Search
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::search::queries::unified_search::handle(pool, query).await }
            }
        })
        // Resolve
        .add_handler({
            let pool = pool.clone();
            move |query| {
                let pool = pool.clone();
                async move { crate::features::resolve::queries::resolve_manifest::handle(pool, query).await }
            }
        })
        // Organisms
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
        // Version Files
        .add_handler({
            let pool = pool.clone();
            move |cmd| {
                let pool = pool.clone();
                async move { crate::features::version_files::commands::add_batch::handle(pool, cmd).await }
            }
        })
        // Protein Metadata
        .add_handler({
            let pool = pool.clone();
            move |cmd| {
                let pool = pool.clone();
                async move { crate::features::protein_metadata::commands::insert::handle(pool, cmd).await }
            }
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mediator_builds() {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost".to_string());

        if let Ok(pool) = PgPool::connect(&database_url).await {
            let _mediator = build_mediator(pool);
        }
    }
}
