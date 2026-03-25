// crates/bdp-ingest/src/pipelines/pubmed/storage.rs

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::pipelines::pubmed::parser::PubmedArticle;

pub struct PubmedStorage {
    pool: PgPool,
    #[allow(dead_code)]
    org_id: Uuid,
}

impl PubmedStorage {
    pub fn new(pool: PgPool, org_id: Uuid) -> Self {
        Self { pool, org_id }
    }

    /// Bulk insert publications. Returns count inserted.
    ///
    /// Uses UNNEST + ON CONFLICT DO NOTHING (pmid is UNIQUE).
    /// After inserting publications, bulk-inserts authors and mesh headings
    /// for each article.
    pub async fn insert_publications_batch(&self, articles: &[PubmedArticle]) -> Result<usize> {
        if articles.is_empty() {
            return Ok(0);
        }

        // --- Build UNNEST arrays for publications ---
        let mut pmids: Vec<i32> = Vec::with_capacity(articles.len());
        let mut pmcids: Vec<Option<String>> = Vec::with_capacity(articles.len());
        let mut dois: Vec<Option<String>> = Vec::with_capacity(articles.len());
        let mut titles: Vec<String> = Vec::with_capacity(articles.len());
        let mut abstracts: Vec<Option<String>> = Vec::with_capacity(articles.len());
        // pub_date: stored as year-only DATE (first day of year), nullable
        let mut pub_dates: Vec<Option<chrono::NaiveDate>> = Vec::with_capacity(articles.len());
        let mut journals: Vec<Option<String>> = Vec::with_capacity(articles.len());

        for art in articles {
            pmids.push(art.pmid);
            pmcids.push(art.pmcid.clone());
            dois.push(art.doi.clone());
            titles.push(art.title.clone());
            abstracts.push(art.abstract_text.clone());
            pub_dates.push(
                art.pub_year
                    .and_then(|y| chrono::NaiveDate::from_ymd_opt(y, 1, 1)),
            );
            journals.push(art.journal.clone());
        }

        // Bulk insert publications; return inserted ids
        let rows = sqlx::query(
            r#"
            INSERT INTO publications (pmid, pmcid, doi, title, abstract, pub_date, journal)
            SELECT * FROM UNNEST(
                $1::int4[],
                $2::text[],
                $3::text[],
                $4::text[],
                $5::text[],
                $6::date[],
                $7::text[]
            ) AS t(pmid, pmcid, doi, title, abstract, pub_date, journal)
            ON CONFLICT (pmid) DO NOTHING
            RETURNING id, pmid
            "#,
        )
        .bind(&pmids)
        .bind(&pmcids)
        .bind(&dois)
        .bind(&titles)
        .bind(&abstracts)
        .bind(&pub_dates)
        .bind(&journals)
        .fetch_all(&self.pool)
        .await?;

        let inserted_count = rows.len();

        // Build a pmid -> id map for newly inserted rows
        use sqlx::Row;
        let mut pmid_to_id: std::collections::HashMap<i32, i64> =
            std::collections::HashMap::with_capacity(rows.len());
        for row in &rows {
            let pub_id: i64 = row.try_get("id")?;
            let pmid: i32 = row.try_get("pmid")?;
            pmid_to_id.insert(pmid, pub_id);
        }

        // --- Bulk insert authors for newly inserted publications ---
        let mut author_pub_ids: Vec<i64> = Vec::new();
        let mut author_positions: Vec<i16> = Vec::new();
        let mut author_last_names: Vec<Option<String>> = Vec::new();
        let mut author_fore_names: Vec<Option<String>> = Vec::new();
        let mut author_collectives: Vec<Option<String>> = Vec::new();
        let mut author_affiliations: Vec<Option<String>> = Vec::new();

        for art in articles {
            if let Some(&pub_id) = pmid_to_id.get(&art.pmid) {
                for (pos, author) in art.authors.iter().enumerate() {
                    author_pub_ids.push(pub_id);
                    author_positions.push(pos as i16 + 1);
                    author_last_names.push(author.last_name.clone());
                    author_fore_names.push(author.fore_name.clone());
                    author_collectives.push(author.collective.clone());
                    author_affiliations.push(author.affiliation.clone());
                }
            }
        }

        if !author_pub_ids.is_empty() {
            sqlx::query(
                r#"
                INSERT INTO publication_authors
                    (publication_id, position, last_name, fore_name, collective, affiliation)
                SELECT * FROM UNNEST(
                    $1::int8[],
                    $2::int2[],
                    $3::text[],
                    $4::text[],
                    $5::text[],
                    $6::text[]
                ) AS t(publication_id, position, last_name, fore_name, collective, affiliation)
                "#,
            )
            .bind(&author_pub_ids)
            .bind(&author_positions)
            .bind(&author_last_names)
            .bind(&author_fore_names)
            .bind(&author_collectives)
            .bind(&author_affiliations)
            .execute(&self.pool)
            .await?;
        }

        // --- Bulk insert mesh headings for newly inserted publications ---
        let mut mesh_pub_ids: Vec<i64> = Vec::new();
        let mut mesh_uis: Vec<String> = Vec::new();
        let mut mesh_descriptors: Vec<String> = Vec::new();
        let mut mesh_major_topics: Vec<bool> = Vec::new();

        for art in articles {
            if let Some(&pub_id) = pmid_to_id.get(&art.pmid) {
                for heading in &art.mesh_headings {
                    mesh_pub_ids.push(pub_id);
                    mesh_uis.push(heading.ui.clone());
                    mesh_descriptors.push(heading.descriptor.clone());
                    mesh_major_topics.push(heading.is_major_topic);
                }
            }
        }

        if !mesh_pub_ids.is_empty() {
            sqlx::query(
                r#"
                INSERT INTO publication_mesh
                    (publication_id, mesh_ui, descriptor, is_major_topic)
                SELECT * FROM UNNEST(
                    $1::int8[],
                    $2::text[],
                    $3::text[],
                    $4::bool[]
                ) AS t(publication_id, mesh_ui, descriptor, is_major_topic)
                "#,
            )
            .bind(&mesh_pub_ids)
            .bind(&mesh_uis)
            .bind(&mesh_descriptors)
            .bind(&mesh_major_topics)
            .execute(&self.pool)
            .await?;
        }

        Ok(inserted_count)
    }

    /// Update pubmed_ingest_files status to 'done' for the given file id.
    pub async fn mark_file_done(&self, file_id: i64) -> Result<()> {
        sqlx::query(
            "UPDATE pubmed_ingest_files SET status = 'done', processed_at = NOW() WHERE id = $1",
        )
        .bind(file_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update pubmed_ingest_files status to 'error' for the given file id.
    pub async fn mark_file_error(&self, file_id: i64, error: &str) -> Result<()> {
        sqlx::query(
            "UPDATE pubmed_ingest_files SET status = 'error', error_message = $1 WHERE id = $2",
        )
        .bind(error)
        .bind(file_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
