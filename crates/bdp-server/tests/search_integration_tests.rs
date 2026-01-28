/// Integration tests for search functionality with materialized view
///
/// These tests verify that the search optimizations work correctly
/// with the materialized view and various filters.
use bdp_server::{
    db::{create_pool, DbConfig},
    features::search::queries::{
        RefreshSearchIndexCommand, SearchSuggestionsQuery, UnifiedSearchQuery,
    },
};
use sqlx::PgPool;

/// Helper to create test data
async fn create_test_data(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    // Create test organizations
    sqlx::query!(
        r#"
        INSERT INTO organizations (slug, name, description, is_system)
        VALUES
            ('test-org', 'Test Organization', 'For testing search', false),
            ('uniprot', 'UniProt', 'Universal Protein Resource', true),
            ('ncbi', 'NCBI', 'National Center for Biotechnology Information', true)
        ON CONFLICT (slug) DO NOTHING
        "#
    )
    .execute(pool)
    .await?;

    // Get organization IDs
    let test_org_id =
        sqlx::query_scalar!(r#"SELECT id FROM organizations WHERE slug = 'test-org'"#)
            .fetch_one(pool)
            .await?;

    let uniprot_id = sqlx::query_scalar!(r#"SELECT id FROM organizations WHERE slug = 'uniprot'"#)
        .fetch_one(pool)
        .await?;

    // Create taxonomy metadata for testing organism filters
    let taxonomy_entry_id = sqlx::query_scalar!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
        VALUES ($1, 'human-taxonomy', 'Human Taxonomy', 'Homo sapiens taxonomy', 'data_source')
        ON CONFLICT (organization_id, slug) DO UPDATE SET name = EXCLUDED.name
        RETURNING id
        "#,
        test_org_id
    )
    .fetch_one(pool)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO data_sources (id, source_type)
        VALUES ($1, 'taxonomy')
        ON CONFLICT (id) DO NOTHING
        "#,
        taxonomy_entry_id
    )
    .execute(pool)
    .await?;

    let taxonomy_ds_id = sqlx::query_scalar!(
        r#"
        INSERT INTO taxonomy_metadata (data_source_id, scientific_name, common_name, taxonomy_id, rank)
        VALUES ($1, 'Homo sapiens', 'Human', 9606, 'species')
        ON CONFLICT (data_source_id) DO UPDATE SET scientific_name = EXCLUDED.scientific_name
        RETURNING data_source_id
        "#,
        taxonomy_entry_id
    )
    .fetch_one(pool)
    .await?;

    // Create protein data sources with different characteristics
    let protein_entries = vec![
        ("insulin-human", "Human Insulin", "Human insulin protein sequence", "protein"),
        ("insulin-mouse", "Mouse Insulin", "Mouse insulin protein sequence", "protein"),
        ("hemoglobin-human", "Human Hemoglobin", "Human hemoglobin protein", "protein"),
        ("collagen-human", "Human Collagen", "Human collagen protein", "protein"),
    ];

    for (slug, name, desc, source_type) in protein_entries {
        let entry_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
            VALUES ($1, $2, $3, $4, 'data_source')
            ON CONFLICT (organization_id, slug) DO UPDATE SET name = EXCLUDED.name
            RETURNING id
            "#,
            uniprot_id,
            slug,
            name,
            desc
        )
        .fetch_one(pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO data_sources (id, source_type)
            VALUES ($1, $2)
            ON CONFLICT (id) DO NOTHING
            "#,
            entry_id,
            source_type
        )
        .execute(pool)
        .await?;

        // Add protein metadata linking to taxonomy
        sqlx::query!(
            r#"
            INSERT INTO protein_metadata (data_source_id, uniprot_id, taxonomy_id, protein_name)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (data_source_id) DO UPDATE SET protein_name = EXCLUDED.protein_name
            "#,
            entry_id,
            format!("P{:05}", entry_id.as_simple().to_string().len()),
            taxonomy_ds_id,
            name
        )
        .execute(pool)
        .await?;

        // Create versions with different formats
        let version_id = sqlx::query_scalar!(
            r#"
            INSERT INTO versions (entry_id, version, external_version, published_at, download_count)
            VALUES ($1, '1.0', 'v1.0', NOW(), 100)
            ON CONFLICT (entry_id, version) DO UPDATE SET download_count = EXCLUDED.download_count
            RETURNING id
            "#,
            entry_id
        )
        .fetch_one(pool)
        .await?;

        // Add file formats
        for format in &["fasta", "json", "xml"] {
            sqlx::query!(
                r#"
                INSERT INTO version_files (version_id, format, file_path, size_bytes, checksum)
                VALUES ($1, $2, $3, 1024, 'abc123')
                ON CONFLICT (version_id, format) DO NOTHING
                "#,
                version_id,
                format,
                format!("/data/{}.{}", slug, format)
            )
            .execute(pool)
            .await?;
        }
    }

    // Create genome data sources
    let genome_entry_id = sqlx::query_scalar!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
        VALUES ($1, 'human-genome', 'Human Genome', 'Complete human genome assembly', 'data_source')
        ON CONFLICT (organization_id, slug) DO UPDATE SET name = EXCLUDED.name
        RETURNING id
        "#,
        test_org_id
    )
    .fetch_one(pool)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO data_sources (id, source_type)
        VALUES ($1, 'genome')
        ON CONFLICT (id) DO NOTHING
        "#,
        genome_entry_id
    )
    .execute(pool)
    .await?;

    // Create version for genome
    let genome_version_id = sqlx::query_scalar!(
        r#"
        INSERT INTO versions (entry_id, version, external_version, published_at, download_count)
        VALUES ($1, '38', 'GRCh38', NOW(), 500)
        ON CONFLICT (entry_id, version) DO UPDATE SET download_count = EXCLUDED.download_count
        RETURNING id
        "#,
        genome_entry_id
    )
    .fetch_one(pool)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO version_files (version_id, format, file_path, size_bytes, checksum)
        VALUES ($1, 'fasta', '/data/genome.fasta', 3000000000, 'def456')
        ON CONFLICT (version_id, format) DO NOTHING
        "#,
        genome_version_id
    )
    .execute(pool)
    .await?;

    // Create a tool entry
    let tool_id = sqlx::query_scalar!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
        VALUES ($1, 'blast', 'BLAST', 'Basic Local Alignment Search Tool', 'tool')
        ON CONFLICT (organization_id, slug) DO UPDATE SET name = EXCLUDED.name
        RETURNING id
        "#,
        test_org_id
    )
    .fetch_one(pool)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO tools (id, tool_type)
        VALUES ($1, 'alignment')
        ON CONFLICT (id) DO NOTHING
        "#,
        tool_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Helper to refresh the materialized view
async fn refresh_search_mv(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let command = RefreshSearchIndexCommand { concurrent: false };
    bdp_server::features::search::queries::refresh_search_index::handle(pool.clone(), command)
        .await?;
    Ok(())
}

#[sqlx::test]
async fn test_search_basic_query(pool: PgPool) -> sqlx::Result<()> {
    create_test_data(&pool).await.unwrap();
    refresh_search_mv(&pool).await.unwrap();

    let query = UnifiedSearchQuery {
        query: "insulin".to_string(),
        type_filter: None,
        source_type_filter: None,
        organism: None,
        format: None,
        page: Some(1),
        per_page: Some(20),
    };

    let response = bdp_server::features::search::queries::unified_search::handle(pool, query)
        .await
        .unwrap();

    assert!(response.items.len() >= 2, "Should find at least 2 insulin entries");
    assert!(response
        .items
        .iter()
        .any(|i| i.name.contains("Human Insulin")));
    assert!(response
        .items
        .iter()
        .any(|i| i.name.contains("Mouse Insulin")));

    Ok(())
}

#[sqlx::test]
async fn test_search_with_type_filter(pool: PgPool) -> sqlx::Result<()> {
    create_test_data(&pool).await.unwrap();
    refresh_search_mv(&pool).await.unwrap();

    // Search only data sources
    let query = UnifiedSearchQuery {
        query: "human".to_string(),
        type_filter: Some(vec!["data_source".to_string()]),
        source_type_filter: None,
        organism: None,
        format: None,
        page: Some(1),
        per_page: Some(20),
    };

    let response =
        bdp_server::features::search::queries::unified_search::handle(pool.clone(), query)
            .await
            .unwrap();

    assert!(response.items.len() > 0, "Should find human data sources");
    assert!(response.items.iter().all(|i| i.entry_type == "data_source"));

    // Search only tools
    let query = UnifiedSearchQuery {
        query: "blast".to_string(),
        type_filter: Some(vec!["tool".to_string()]),
        source_type_filter: None,
        organism: None,
        format: None,
        page: Some(1),
        per_page: Some(20),
    };

    let response = bdp_server::features::search::queries::unified_search::handle(pool, query)
        .await
        .unwrap();

    assert_eq!(response.items.len(), 1, "Should find BLAST tool");
    assert_eq!(response.items[0].entry_type, "tool");
    assert_eq!(response.items[0].name, "BLAST");

    Ok(())
}

#[sqlx::test]
async fn test_search_with_source_type_filter(pool: PgPool) -> sqlx::Result<()> {
    create_test_data(&pool).await.unwrap();
    refresh_search_mv(&pool).await.unwrap();

    // Search only protein sources
    let query = UnifiedSearchQuery {
        query: "human".to_string(),
        type_filter: Some(vec!["data_source".to_string()]),
        source_type_filter: Some(vec!["protein".to_string()]),
        organism: None,
        format: None,
        page: Some(1),
        per_page: Some(20),
    };

    let response =
        bdp_server::features::search::queries::unified_search::handle(pool.clone(), query)
            .await
            .unwrap();

    assert!(response.items.len() > 0, "Should find protein sources");
    assert!(response
        .items
        .iter()
        .all(|i| i.source_type.as_deref() == Some("protein")));

    // Search only genome sources
    let query = UnifiedSearchQuery {
        query: "genome".to_string(),
        type_filter: Some(vec!["data_source".to_string()]),
        source_type_filter: Some(vec!["genome".to_string()]),
        organism: None,
        format: None,
        page: Some(1),
        per_page: Some(20),
    };

    let response = bdp_server::features::search::queries::unified_search::handle(pool, query)
        .await
        .unwrap();

    assert_eq!(response.items.len(), 1, "Should find genome source");
    assert_eq!(response.items[0].source_type.as_deref(), Some("genome"));

    Ok(())
}

#[sqlx::test]
async fn test_search_with_organism_filter(pool: PgPool) -> sqlx::Result<()> {
    create_test_data(&pool).await.unwrap();
    refresh_search_mv(&pool).await.unwrap();

    // Search by scientific name
    let query = UnifiedSearchQuery {
        query: "insulin".to_string(),
        type_filter: Some(vec!["data_source".to_string()]),
        source_type_filter: None,
        organism: Some("Homo sapiens".to_string()),
        format: None,
        page: Some(1),
        per_page: Some(20),
    };

    let response =
        bdp_server::features::search::queries::unified_search::handle(pool.clone(), query)
            .await
            .unwrap();

    assert!(response.items.len() > 0, "Should find human insulin");
    assert!(response.items.iter().all(|i| {
        i.organism
            .as_ref()
            .map_or(false, |o| o.scientific_name.contains("sapiens"))
    }));

    // Search by common name
    let query = UnifiedSearchQuery {
        query: "insulin".to_string(),
        type_filter: Some(vec!["data_source".to_string()]),
        source_type_filter: None,
        organism: Some("human".to_string()),
        format: None,
        page: Some(1),
        per_page: Some(20),
    };

    let response = bdp_server::features::search::queries::unified_search::handle(pool, query)
        .await
        .unwrap();

    assert!(response.items.len() > 0, "Should find human insulin by common name");

    Ok(())
}

#[sqlx::test]
async fn test_search_with_format_filter(pool: PgPool) -> sqlx::Result<()> {
    create_test_data(&pool).await.unwrap();
    refresh_search_mv(&pool).await.unwrap();

    // Search for entries with FASTA format
    let query = UnifiedSearchQuery {
        query: "insulin".to_string(),
        type_filter: None,
        source_type_filter: None,
        organism: None,
        format: Some("fasta".to_string()),
        page: Some(1),
        per_page: Some(20),
    };

    let response =
        bdp_server::features::search::queries::unified_search::handle(pool.clone(), query)
            .await
            .unwrap();

    assert!(response.items.len() > 0, "Should find entries with FASTA format");
    assert!(response
        .items
        .iter()
        .all(|i| i.available_formats.contains(&"fasta".to_string())));

    // Search for entries with JSON format
    let query = UnifiedSearchQuery {
        query: "human".to_string(),
        type_filter: Some(vec!["data_source".to_string()]),
        source_type_filter: None,
        organism: None,
        format: Some("json".to_string()),
        page: Some(1),
        per_page: Some(20),
    };

    let response = bdp_server::features::search::queries::unified_search::handle(pool, query)
        .await
        .unwrap();

    assert!(response.items.len() > 0, "Should find entries with JSON format");
    assert!(response
        .items
        .iter()
        .all(|i| i.available_formats.contains(&"json".to_string())));

    Ok(())
}

#[sqlx::test]
async fn test_search_pagination(pool: PgPool) -> sqlx::Result<()> {
    create_test_data(&pool).await.unwrap();
    refresh_search_mv(&pool).await.unwrap();

    // Page 1 with 2 items per page
    let query = UnifiedSearchQuery {
        query: "human".to_string(),
        type_filter: Some(vec!["data_source".to_string()]),
        source_type_filter: None,
        organism: None,
        format: None,
        page: Some(1),
        per_page: Some(2),
    };

    let page1 = bdp_server::features::search::queries::unified_search::handle(pool.clone(), query)
        .await
        .unwrap();

    assert!(page1.items.len() <= 2, "Page 1 should have at most 2 items");
    assert_eq!(page1.pagination.page, 1);
    assert_eq!(page1.pagination.per_page, 2);
    assert!(page1.pagination.total >= 2, "Should have at least 2 total items");

    // Page 2
    let query = UnifiedSearchQuery {
        query: "human".to_string(),
        type_filter: Some(vec!["data_source".to_string()]),
        source_type_filter: None,
        organism: None,
        format: None,
        page: Some(2),
        per_page: Some(2),
    };

    let page2 = bdp_server::features::search::queries::unified_search::handle(pool, query)
        .await
        .unwrap();

    assert_eq!(page2.pagination.page, 2);

    // Verify no overlap between pages
    if !page1.items.is_empty() && !page2.items.is_empty() {
        let page1_ids: Vec<_> = page1.items.iter().map(|i| i.id).collect();
        let page2_ids: Vec<_> = page2.items.iter().map(|i| i.id).collect();
        assert!(page1_ids.iter().all(|id| !page2_ids.contains(id)), "Pages should not overlap");
    }

    Ok(())
}

#[sqlx::test]
async fn test_search_ranking(pool: PgPool) -> sqlx::Result<()> {
    create_test_data(&pool).await.unwrap();
    refresh_search_mv(&pool).await.unwrap();

    let query = UnifiedSearchQuery {
        query: "insulin".to_string(),
        type_filter: None,
        source_type_filter: None,
        organism: None,
        format: None,
        page: Some(1),
        per_page: Some(20),
    };

    let response = bdp_server::features::search::queries::unified_search::handle(pool, query)
        .await
        .unwrap();

    // Verify results are sorted by rank
    for i in 0..response.items.len().saturating_sub(1) {
        assert!(
            response.items[i].rank >= response.items[i + 1].rank,
            "Results should be sorted by rank descending"
        );
    }

    Ok(())
}

#[sqlx::test]
async fn test_search_precomputed_fields(pool: PgPool) -> sqlx::Result<()> {
    create_test_data(&pool).await.unwrap();
    refresh_search_mv(&pool).await.unwrap();

    let query = UnifiedSearchQuery {
        query: "insulin".to_string(),
        type_filter: Some(vec!["data_source".to_string()]),
        source_type_filter: None,
        organism: None,
        format: None,
        page: Some(1),
        per_page: Some(20),
    };

    let response = bdp_server::features::search::queries::unified_search::handle(pool, query)
        .await
        .unwrap();

    for item in response.items {
        // Verify pre-computed fields are present
        assert!(item.latest_version.is_some(), "Should have latest_version");
        assert!(!item.available_formats.is_empty(), "Should have available_formats");
        assert!(item.total_downloads >= 0, "Should have total_downloads");

        if item.source_type.as_deref() == Some("protein") {
            assert!(item.organism.is_some(), "Protein should have organism info");
        }
    }

    Ok(())
}

#[sqlx::test]
async fn test_suggestions_basic(pool: PgPool) -> sqlx::Result<()> {
    create_test_data(&pool).await.unwrap();
    refresh_search_mv(&pool).await.unwrap();

    let query = SearchSuggestionsQuery {
        q: "ins".to_string(),
        limit: Some(10),
        type_filter: None,
        source_type_filter: None,
    };

    let response = bdp_server::features::search::queries::suggestions::handle(pool, query)
        .await
        .unwrap();

    assert!(response.suggestions.len() > 0, "Should find suggestions for 'ins'");
    assert!(response
        .suggestions
        .iter()
        .any(|s| s.name.to_lowercase().contains("insulin")));

    Ok(())
}

#[sqlx::test]
async fn test_suggestions_with_filters(pool: PgPool) -> sqlx::Result<()> {
    create_test_data(&pool).await.unwrap();
    refresh_search_mv(&pool).await.unwrap();

    // Filter by source type
    let query = SearchSuggestionsQuery {
        q: "human".to_string(),
        limit: Some(10),
        type_filter: Some(vec!["data_source".to_string()]),
        source_type_filter: Some(vec!["protein".to_string()]),
    };

    let response = bdp_server::features::search::queries::suggestions::handle(pool, query)
        .await
        .unwrap();

    assert!(response.suggestions.len() > 0, "Should find protein suggestions");
    assert!(response
        .suggestions
        .iter()
        .all(|s| s.source_type.as_deref() == Some("protein")));

    Ok(())
}

#[sqlx::test]
async fn test_suggestions_limit(pool: PgPool) -> sqlx::Result<()> {
    create_test_data(&pool).await.unwrap();
    refresh_search_mv(&pool).await.unwrap();

    let query = SearchSuggestionsQuery {
        q: "hu".to_string(),
        limit: Some(3),
        type_filter: None,
        source_type_filter: None,
    };

    let response = bdp_server::features::search::queries::suggestions::handle(pool, query)
        .await
        .unwrap();

    assert!(response.suggestions.len() <= 3, "Should respect limit");

    Ok(())
}

#[sqlx::test]
async fn test_materialized_view_refresh(pool: PgPool) -> sqlx::Result<()> {
    create_test_data(&pool).await.unwrap();

    // Initial refresh
    refresh_search_mv(&pool).await.unwrap();

    // Count initial entries
    let initial_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*)::bigint as \"count!\" FROM search_registry_entries_mv"
    )
    .fetch_one(&pool)
    .await?;

    // Add a new entry
    let org_id = sqlx::query_scalar!(r#"SELECT id FROM organizations WHERE slug = 'test-org'"#)
        .fetch_one(&pool)
        .await?;

    let new_entry_id = sqlx::query_scalar!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, description, entry_type)
        VALUES ($1, 'new-test-entry', 'New Test Entry', 'Testing MV refresh', 'data_source')
        RETURNING id
        "#,
        org_id
    )
    .fetch_one(&pool)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO data_sources (id, source_type)
        VALUES ($1, 'protein')
        "#,
        new_entry_id
    )
    .execute(&pool)
    .await?;

    // Refresh MV
    refresh_search_mv(&pool).await.unwrap();

    // Count after refresh
    let new_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*)::bigint as \"count!\" FROM search_registry_entries_mv"
    )
    .fetch_one(&pool)
    .await?;

    assert!(new_count > initial_count, "MV should include new entry after refresh");

    Ok(())
}

#[sqlx::test]
async fn test_combined_filters(pool: PgPool) -> sqlx::Result<()> {
    create_test_data(&pool).await.unwrap();
    refresh_search_mv(&pool).await.unwrap();

    // Combine multiple filters
    let query = UnifiedSearchQuery {
        query: "human".to_string(),
        type_filter: Some(vec!["data_source".to_string()]),
        source_type_filter: Some(vec!["protein".to_string()]),
        organism: Some("sapiens".to_string()),
        format: Some("fasta".to_string()),
        page: Some(1),
        per_page: Some(20),
    };

    let response = bdp_server::features::search::queries::unified_search::handle(pool, query)
        .await
        .unwrap();

    // Verify all filters are applied
    for item in response.items {
        assert_eq!(item.entry_type, "data_source");
        assert_eq!(item.source_type.as_deref(), Some("protein"));
        assert!(item
            .organism
            .as_ref()
            .map_or(false, |o| o.scientific_name.contains("sapiens")));
        assert!(item.available_formats.contains(&"fasta".to_string()));
    }

    Ok(())
}
