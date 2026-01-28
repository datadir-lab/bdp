use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertProteinMetadataCommand {
    pub data_source_id: Uuid,
    pub accession: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protein_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gene_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_length: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mass_da: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertProteinMetadataResponse {
    pub data_source_id: Uuid,
    pub accession: String,
}

#[derive(Debug, thiserror::Error)]
pub enum InsertProteinMetadataError {
    #[error("Accession is required and cannot be empty")]
    AccessionRequired,
    #[error("Accession must be between 1 and 50 characters")]
    AccessionLength,
    #[error("Accession must only contain alphanumeric characters")]
    AccessionFormat,
    #[error("Entry name must not exceed 255 characters")]
    EntryNameLength,
    #[error("Gene name must not exceed 255 characters")]
    GeneNameLength,
    #[error("Sequence checksum must not exceed 64 characters")]
    ChecksumLength,
    #[error("Data source with ID '{0}' not found")]
    DataSourceNotFound(Uuid),
    #[error("Protein metadata with accession '{0}' already exists")]
    DuplicateAccession(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<InsertProteinMetadataResponse, InsertProteinMetadataError>>
    for InsertProteinMetadataCommand
{
}

impl crate::cqrs::middleware::Command for InsertProteinMetadataCommand {}

impl InsertProteinMetadataCommand {
    pub fn validate(&self) -> Result<(), InsertProteinMetadataError> {
        if self.accession.trim().is_empty() {
            return Err(InsertProteinMetadataError::AccessionRequired);
        }
        if self.accession.len() > 50 {
            return Err(InsertProteinMetadataError::AccessionLength);
        }
        if !self.accession.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(InsertProteinMetadataError::AccessionFormat);
        }
        if let Some(ref entry_name) = self.entry_name {
            if entry_name.len() > 255 {
                return Err(InsertProteinMetadataError::EntryNameLength);
            }
        }
        if let Some(ref gene_name) = self.gene_name {
            if gene_name.len() > 255 {
                return Err(InsertProteinMetadataError::GeneNameLength);
            }
        }
        if let Some(ref checksum) = self.sequence_checksum {
            if checksum.len() > 64 {
                return Err(InsertProteinMetadataError::ChecksumLength);
            }
        }
        Ok(())
    }
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    command: InsertProteinMetadataCommand,
) -> Result<InsertProteinMetadataResponse, InsertProteinMetadataError> {
    command.validate()?;

    // Check if data source exists
    let data_source_exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM data_sources WHERE id = $1
        )
        "#,
        command.data_source_id
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(false);

    if !data_source_exists {
        return Err(InsertProteinMetadataError::DataSourceNotFound(command.data_source_id));
    }

    // Insert or update protein metadata
    sqlx::query!(
        r#"
        INSERT INTO protein_metadata
            (data_source_id, accession, entry_name, protein_name, gene_name,
             sequence_length, mass_da, sequence_checksum)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (accession) DO UPDATE SET
            data_source_id = EXCLUDED.data_source_id,
            entry_name = EXCLUDED.entry_name,
            protein_name = EXCLUDED.protein_name,
            gene_name = EXCLUDED.gene_name,
            sequence_length = EXCLUDED.sequence_length,
            mass_da = EXCLUDED.mass_da,
            sequence_checksum = EXCLUDED.sequence_checksum
        "#,
        command.data_source_id,
        command.accession,
        command.entry_name,
        command.protein_name,
        command.gene_name,
        command.sequence_length,
        command.mass_da,
        command.sequence_checksum
    )
    .execute(&pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.is_foreign_key_violation() {
                return InsertProteinMetadataError::DataSourceNotFound(command.data_source_id);
            }
        }
        InsertProteinMetadataError::Database(e)
    })?;

    Ok(InsertProteinMetadataResponse {
        data_source_id: command.data_source_id,
        accession: command.accession,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let cmd = InsertProteinMetadataCommand {
            data_source_id: Uuid::new_v4(),
            accession: "P01308".to_string(),
            entry_name: Some("INS_HUMAN".to_string()),
            protein_name: Some("Insulin".to_string()),
            gene_name: Some("INS".to_string()),
            sequence_length: Some(110),
            mass_da: Some(11937),
            sequence_checksum: Some("6F2B89D7AAAC28AC".to_string()),
        };
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_accession() {
        let cmd = InsertProteinMetadataCommand {
            data_source_id: Uuid::new_v4(),
            accession: "".to_string(),
            entry_name: None,
            protein_name: None,
            gene_name: None,
            sequence_length: None,
            mass_da: None,
            sequence_checksum: None,
        };
        assert!(matches!(cmd.validate(), Err(InsertProteinMetadataError::AccessionRequired)));
    }

    #[test]
    fn test_validation_invalid_accession_format() {
        let cmd = InsertProteinMetadataCommand {
            data_source_id: Uuid::new_v4(),
            accession: "P01308-INVALID".to_string(),
            entry_name: None,
            protein_name: None,
            gene_name: None,
            sequence_length: None,
            mass_da: None,
            sequence_checksum: None,
        };
        assert!(matches!(cmd.validate(), Err(InsertProteinMetadataError::AccessionFormat)));
    }

    #[test]
    fn test_validation_accession_too_long() {
        let cmd = InsertProteinMetadataCommand {
            data_source_id: Uuid::new_v4(),
            accession: "A".repeat(51),
            entry_name: None,
            protein_name: None,
            gene_name: None,
            sequence_length: None,
            mass_da: None,
            sequence_checksum: None,
        };
        assert!(matches!(cmd.validate(), Err(InsertProteinMetadataError::AccessionLength)));
    }

    #[test]
    fn test_validation_accession_max_length() {
        let cmd = InsertProteinMetadataCommand {
            data_source_id: Uuid::new_v4(),
            accession: "A".repeat(50),
            entry_name: None,
            protein_name: None,
            gene_name: None,
            sequence_length: None,
            mass_da: None,
            sequence_checksum: None,
        };
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validation_whitespace_accession() {
        let cmd = InsertProteinMetadataCommand {
            data_source_id: Uuid::new_v4(),
            accession: "   ".to_string(),
            entry_name: None,
            protein_name: None,
            gene_name: None,
            sequence_length: None,
            mass_da: None,
            sequence_checksum: None,
        };
        assert!(matches!(cmd.validate(), Err(InsertProteinMetadataError::AccessionRequired)));
    }

    #[test]
    fn test_validation_entry_name_max_length() {
        let cmd = InsertProteinMetadataCommand {
            data_source_id: Uuid::new_v4(),
            accession: "P01308".to_string(),
            entry_name: Some("A".repeat(255)),
            protein_name: None,
            gene_name: None,
            sequence_length: None,
            mass_da: None,
            sequence_checksum: None,
        };
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validation_entry_name_too_long() {
        let cmd = InsertProteinMetadataCommand {
            data_source_id: Uuid::new_v4(),
            accession: "P01308".to_string(),
            entry_name: Some("A".repeat(256)),
            protein_name: None,
            gene_name: None,
            sequence_length: None,
            mass_da: None,
            sequence_checksum: None,
        };
        assert!(matches!(cmd.validate(), Err(InsertProteinMetadataError::EntryNameLength)));
    }

    #[test]
    fn test_validation_gene_name_max_length() {
        let cmd = InsertProteinMetadataCommand {
            data_source_id: Uuid::new_v4(),
            accession: "P01308".to_string(),
            entry_name: None,
            protein_name: None,
            gene_name: Some("A".repeat(255)),
            sequence_length: None,
            mass_da: None,
            sequence_checksum: None,
        };
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validation_gene_name_too_long() {
        let cmd = InsertProteinMetadataCommand {
            data_source_id: Uuid::new_v4(),
            accession: "P01308".to_string(),
            entry_name: None,
            protein_name: None,
            gene_name: Some("A".repeat(256)),
            sequence_length: None,
            mass_da: None,
            sequence_checksum: None,
        };
        assert!(matches!(cmd.validate(), Err(InsertProteinMetadataError::GeneNameLength)));
    }

    #[test]
    fn test_validation_checksum_max_length() {
        let cmd = InsertProteinMetadataCommand {
            data_source_id: Uuid::new_v4(),
            accession: "P01308".to_string(),
            entry_name: None,
            protein_name: None,
            gene_name: None,
            sequence_length: None,
            mass_da: None,
            sequence_checksum: Some("a".repeat(64)),
        };
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validation_checksum_too_long() {
        let cmd = InsertProteinMetadataCommand {
            data_source_id: Uuid::new_v4(),
            accession: "P01308".to_string(),
            entry_name: None,
            protein_name: None,
            gene_name: None,
            sequence_length: None,
            mass_da: None,
            sequence_checksum: Some("a".repeat(65)),
        };
        assert!(matches!(cmd.validate(), Err(InsertProteinMetadataError::ChecksumLength)));
    }

    #[test]
    fn test_validation_minimal_valid() {
        let cmd = InsertProteinMetadataCommand {
            data_source_id: Uuid::new_v4(),
            accession: "A".to_string(),
            entry_name: None,
            protein_name: None,
            gene_name: None,
            sequence_length: None,
            mass_da: None,
            sequence_checksum: None,
        };
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_validation_numbers_in_accession() {
        let cmd = InsertProteinMetadataCommand {
            data_source_id: Uuid::new_v4(),
            accession: "P01308".to_string(),
            entry_name: None,
            protein_name: None,
            gene_name: None,
            sequence_length: None,
            mass_da: None,
            sequence_checksum: None,
        };
        assert!(cmd.validate().is_ok());
    }

    #[sqlx::test]
    async fn test_handle_inserts_protein_metadata(pool: PgPool) -> sqlx::Result<()> {
        // Setup: create organization, registry entry, and data source
        let org_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, $2, $3, $4)",
            org_id,
            "uniprot",
            "UniProt",
            true
        )
        .execute(&pool)
        .await?;

        let entry_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, 'data_source')
            RETURNING id
            "#,
            org_id,
            "p01308",
            "Insulin"
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
            entry_id,
            "protein"
        )
        .execute(&pool)
        .await?;

        let cmd = InsertProteinMetadataCommand {
            data_source_id: entry_id,
            accession: "P01308".to_string(),
            entry_name: Some("INS_HUMAN".to_string()),
            protein_name: Some("Insulin".to_string()),
            gene_name: Some("INS".to_string()),
            sequence_length: Some(110),
            mass_da: Some(11937),
            sequence_checksum: Some("6F2B89D7AAAC28AC".to_string()),
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.accession, "P01308");
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_data_source_not_found(pool: PgPool) -> sqlx::Result<()> {
        let cmd = InsertProteinMetadataCommand {
            data_source_id: Uuid::new_v4(),
            accession: "P01308".to_string(),
            entry_name: None,
            protein_name: None,
            gene_name: None,
            sequence_length: None,
            mass_da: None,
            sequence_checksum: None,
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(matches!(result, Err(InsertProteinMetadataError::DataSourceNotFound(_))));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_upserts_on_conflict(pool: PgPool) -> sqlx::Result<()> {
        // Setup
        let org_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, $2, $3, $4)",
            org_id,
            "uniprot",
            "UniProt",
            true
        )
        .execute(&pool)
        .await?;

        let entry_id1 = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, 'data_source')
            RETURNING id
            "#,
            org_id,
            "p01308-v1",
            "Insulin v1"
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
            entry_id1,
            "protein"
        )
        .execute(&pool)
        .await?;

        // First insert
        let cmd1 = InsertProteinMetadataCommand {
            data_source_id: entry_id1,
            accession: "P01308".to_string(),
            entry_name: Some("INS_HUMAN".to_string()),
            protein_name: Some("Insulin".to_string()),
            gene_name: Some("INS".to_string()),
            sequence_length: Some(110),
            mass_da: Some(11937),
            sequence_checksum: None,
        };
        let _ = handle(pool.clone(), cmd1).await.unwrap();

        // Create second data source with same accession
        let entry_id2 = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, 'data_source')
            RETURNING id
            "#,
            org_id,
            "p01308-v2",
            "Insulin v2"
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
            entry_id2,
            "protein"
        )
        .execute(&pool)
        .await?;

        // Update with new data source
        let cmd2 = InsertProteinMetadataCommand {
            data_source_id: entry_id2,
            accession: "P01308".to_string(),
            entry_name: Some("INS_HUMAN_UPDATED".to_string()),
            protein_name: Some("Insulin updated".to_string()),
            gene_name: Some("INS".to_string()),
            sequence_length: Some(110),
            mass_da: Some(11937),
            sequence_checksum: Some("NEW_CHECKSUM".to_string()),
        };
        let result = handle(pool.clone(), cmd2).await;
        assert!(result.is_ok());

        // Verify upsert
        let record = sqlx::query!(
            "SELECT entry_name, data_source_id FROM protein_metadata WHERE accession = 'P01308'"
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(record.entry_name, Some("INS_HUMAN_UPDATED".to_string()));
        assert_eq!(record.data_source_id, entry_id2);
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_minimal_protein_metadata(pool: PgPool) -> sqlx::Result<()> {
        let org_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, $2, $3, $4)",
            org_id,
            "test-org",
            "Test Org",
            false
        )
        .execute(&pool)
        .await?;

        let entry_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, 'data_source')
            RETURNING id
            "#,
            org_id,
            "test-ds",
            "Test DS"
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
            entry_id,
            "protein"
        )
        .execute(&pool)
        .await?;

        let cmd = InsertProteinMetadataCommand {
            data_source_id: entry_id,
            accession: "TESTACCESSION".to_string(),
            entry_name: None,
            protein_name: None,
            gene_name: None,
            sequence_length: None,
            mass_da: None,
            sequence_checksum: None,
        };

        let result = handle(pool.clone(), cmd).await;
        assert!(result.is_ok());
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_multiple_proteins(pool: PgPool) -> sqlx::Result<()> {
        let org_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO organizations (id, slug, name, is_system) VALUES ($1, $2, $3, $4)",
            org_id,
            "test-org",
            "Test Org",
            false
        )
        .execute(&pool)
        .await?;

        let entry_id = sqlx::query_scalar!(
            r#"
            INSERT INTO registry_entries (organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, 'data_source')
            RETURNING id
            "#,
            org_id,
            "test-ds",
            "Test DS"
        )
        .fetch_one(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO data_sources (id, source_type) VALUES ($1, $2)",
            entry_id,
            "protein"
        )
        .execute(&pool)
        .await?;

        let cmd1 = InsertProteinMetadataCommand {
            data_source_id: entry_id,
            accession: "P01308".to_string(),
            entry_name: Some("INS_HUMAN".to_string()),
            protein_name: None,
            gene_name: None,
            sequence_length: None,
            mass_da: None,
            sequence_checksum: None,
        };
        let result1 = handle(pool.clone(), cmd1).await.unwrap();

        let cmd2 = InsertProteinMetadataCommand {
            data_source_id: entry_id,
            accession: "Q12345".to_string(),
            entry_name: Some("TEST_HUMAN".to_string()),
            protein_name: None,
            gene_name: None,
            sequence_length: None,
            mass_da: None,
            sequence_checksum: None,
        };
        let result2 = handle(pool.clone(), cmd2).await.unwrap();

        assert_ne!(result1.accession, result2.accession);
        Ok(())
    }
}
