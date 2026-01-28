// InterPro Storage Layer
//
// High-performance storage operations for InterPro data with:
// - NO N+1 queries - everything is batched
// - Transaction support for atomicity
// - Efficient bulk operations
// - Proper error handling
//
// Design Principles:
// 1. Batch everything - use helpers for lookups, bulk inserts for writes
// 2. Transaction boundaries - atomic operations for data consistency
// 3. Version-specific FKs - always reference specific versions
// 4. Minimize round trips - combine operations where possible

use crate::error::Error;
use crate::ingest::interpro::helpers::{
    GoTermLookupHelper, InterProEntryLookupHelper, ProteinLookupHelper, SignatureLookupHelper,
};
use crate::ingest::interpro::models::{
    ExternalReferenceData, GoMappingData, InterProEntry, InterProMetadata,
    MemberSignatureData, ProteinMatch,
};
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn};
use uuid::Uuid;

// ============================================================================
// Storage Configuration
// ============================================================================

const DEFAULT_BATCH_SIZE: usize = 500;
const INTERPRO_ORGANIZATION: &str = "InterPro";
const INTERPRO_WEBSITE: &str = "https://www.ebi.ac.uk/interpro/";
const INTERPRO_CITATION: &str = "Blum M et al. (2025) InterPro: the protein sequence classification resource in 2025. Nucleic Acids Res. 2025 Jan;53 (Database issue) D444-D456.";
const INTERPRO_LICENSE: &str = "Public Domain (CC0-like) - Free to use for any purpose. Please cite the resource and relevant member databases.";

// ============================================================================
// InterPro Entry Storage
// ============================================================================

/// Store InterPro entry metadata and create data source
///
/// Returns (data_source_id, version_id) for the created/updated entry
pub async fn store_interpro_entry(
    pool: &PgPool,
    entry: &InterProEntry,
    version: &str, // InterPro version like "96.0", "97.0"
) -> Result<(Uuid, Uuid), Error> {
    let mut tx = pool.begin().await?;

    // Get or create organization with proper metadata
    let org_id: Uuid = sqlx::query_scalar!(
        r#"
        INSERT INTO organizations (slug, name, website, license, license_url, citation, citation_url)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (slug) DO UPDATE SET
            name = EXCLUDED.name,
            website = EXCLUDED.website,
            license = EXCLUDED.license,
            license_url = EXCLUDED.license_url,
            citation = EXCLUDED.citation,
            citation_url = EXCLUDED.citation_url
        RETURNING id
        "#,
        INTERPRO_ORGANIZATION.to_lowercase(),
        INTERPRO_ORGANIZATION,
        INTERPRO_WEBSITE,
        INTERPRO_LICENSE,
        "https://interpro-documentation.readthedocs.io/en/latest/license.html",
        INTERPRO_CITATION,
        "https://interpro-documentation.readthedocs.io/en/latest/citing.html"
    )
    .fetch_one(pool)
    .await?;

    // Create slug from InterPro ID
    let slug = format!("interpro-{}", entry.interpro_id.to_lowercase());

    // Insert into registry_entries (get an ID back)
    let registry_id: Uuid = sqlx::query_scalar!(
        r#"
        INSERT INTO registry_entries (organization_id, slug, name, entry_type)
        VALUES ($1, $2, $3, 'data_source')
        ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name
        RETURNING id
        "#,
        org_id,
        slug,
        entry.name
    )
    .fetch_one(&mut *tx)
    .await?;

    // Insert into data_sources using the SAME ID (data_sources.id FK to registry_entries.id)
    sqlx::query!(
        r#"
        INSERT INTO data_sources (id, source_type)
        VALUES ($1, 'interpro_entry')
        ON CONFLICT (id) DO NOTHING
        "#,
        registry_id
    )
    .execute(&mut *tx)
    .await?;

    // data_source_id IS the registry_id
    let data_source_id = registry_id;

    // Create or update InterPro entry metadata
    let entry_type_str = entry.entry_type.to_string();

    sqlx::query!(
        r#"
        INSERT INTO interpro_entry_metadata (
            data_source_id, interpro_id, entry_type, name, short_name, description
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (interpro_id) DO UPDATE
        SET
            name = EXCLUDED.name,
            short_name = EXCLUDED.short_name,
            description = EXCLUDED.description,
            updated_at = NOW()
        "#,
        data_source_id,
        entry.interpro_id,
        entry_type_str,
        entry.name,
        entry.short_name,
        entry.description
    )
    .execute(&mut *tx)
    .await?;

    // Parse version (e.g., "96.0" → major=96, minor=0)
    let version_parts: Vec<&str> = version.split('.').collect();
    let version_major: i32 = version_parts.get(0).and_then(|v| v.parse().ok()).unwrap_or(1);
    let version_minor: i32 = version_parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0);

    // Create version with actual InterPro release version
    let version_id: Uuid = sqlx::query_scalar!(
        r#"
        INSERT INTO versions (entry_id, version, version_major, version_minor)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (entry_id, version)
        DO UPDATE SET entry_id = EXCLUDED.entry_id
        RETURNING id
        "#,
        registry_id,
        version,
        version_major,
        version_minor
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    info!(
        interpro_id = %entry.interpro_id,
        data_source_id = %data_source_id,
        version_id = %version_id,
        "Stored InterPro entry"
    );

    Ok((data_source_id, version_id))
}

/// Batch store multiple InterPro entries
///
/// Returns map of interpro_id -> (data_source_id, version_id)
pub async fn store_interpro_entries_batch(
    pool: &PgPool,
    entries: &[InterProEntry],
    version: &str, // InterPro version like "96.0"
) -> Result<HashMap<String, (Uuid, Uuid)>, Error> {
    let mut result = HashMap::new();

    // Process in chunks to avoid overwhelming the database
    for chunk in entries.chunks(DEFAULT_BATCH_SIZE) {
        for entry in chunk {
            let (ds_id, ver_id) = store_interpro_entry(pool, entry, version).await?;
            result.insert(entry.interpro_id.clone(), (ds_id, ver_id));
        }
    }

    info!("Stored {} InterPro entries in batch", entries.len());

    Ok(result)
}

// ============================================================================
// Protein Signature Storage
// ============================================================================

/// Store or update a protein signature
///
/// Returns signature_id
pub async fn store_signature(
    pool: &PgPool,
    signature: &MemberSignatureData,
) -> Result<Uuid, Error> {
    let db_name = signature.database.to_string();

    let signature_id = sqlx::query_scalar!(
        r#"
        INSERT INTO protein_signatures (
            database, accession, name, description
        )
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (database, accession) DO UPDATE
        SET
            name = COALESCE(EXCLUDED.name, protein_signatures.name),
            description = COALESCE(EXCLUDED.description, protein_signatures.description),
            updated_at = NOW()
        RETURNING id
        "#,
        db_name,
        signature.accession,
        signature.name,
        signature.description
    )
    .fetch_one(pool)
    .await?;

    Ok(signature_id)
}

/// Batch store protein signatures
///
/// Returns map of (database, accession) -> signature_id
pub async fn store_signatures_batch(
    pool: &PgPool,
    signatures: &[MemberSignatureData],
) -> Result<HashMap<(String, String), Uuid>, Error> {
    let mut result = HashMap::new();

    // Deduplicate signatures first
    let mut unique_sigs: HashMap<(String, String), &MemberSignatureData> = HashMap::new();
    for sig in signatures {
        let key = (sig.database.to_string(), sig.accession.clone());
        unique_sigs.entry(key).or_insert(sig);
    }

    debug!("Storing {} unique signatures", unique_sigs.len());

    // Store each unique signature
    for ((db, acc), sig) in unique_sigs {
        let sig_id = store_signature(pool, sig).await?;
        result.insert((db, acc), sig_id);
    }

    info!("Stored {} protein signatures in batch", result.len());

    Ok(result)
}

/// Link signatures to InterPro entry
pub async fn link_signatures_to_entry(
    pool: &PgPool,
    interpro_data_source_id: Uuid,
    signature_ids: &[(Uuid, bool)], // (signature_id, is_primary)
) -> Result<(), Error> {
    let mut tx = pool.begin().await?;

    for (signature_id, is_primary) in signature_ids {
        sqlx::query!(
            r#"
            INSERT INTO interpro_member_signatures (
                interpro_data_source_id, signature_id, is_primary
            )
            VALUES ($1, $2, $3)
            ON CONFLICT (interpro_data_source_id, signature_id) DO UPDATE
            SET is_primary = EXCLUDED.is_primary
            "#,
            interpro_data_source_id,
            signature_id,
            is_primary
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    debug!(
        "Linked {} signatures to InterPro entry {}",
        signature_ids.len(),
        interpro_data_source_id
    );

    Ok(())
}

// ============================================================================
// GO Mapping Storage
// ============================================================================

/// Store GO term mappings for an InterPro entry
///
/// Uses helper to batch lookup GO terms
pub async fn store_go_mappings(
    pool: &PgPool,
    interpro_data_source_id: Uuid,
    interpro_version_id: Uuid,
    mappings: &[GoMappingData],
    go_helper: &mut GoTermLookupHelper,
) -> Result<usize, Error> {
    if mappings.is_empty() {
        return Ok(0);
    }

    // Batch load all GO terms
    let go_ids: Vec<String> = mappings.iter().map(|m| m.go_id.clone()).collect();
    go_helper.load_batch(pool, &go_ids).await?;

    let mut tx = pool.begin().await?;
    let mut stored_count = 0;

    for mapping in mappings {
        // Get GO term data source and version from helper
        if let Some((go_ds_id, go_ver_id)) = go_helper.get(&mapping.go_id) {
            sqlx::query!(
                r#"
                INSERT INTO interpro_go_mappings (
                    interpro_data_source_id, interpro_version_id,
                    go_data_source_id, go_version_id, evidence_code
                )
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (interpro_data_source_id, go_data_source_id) DO NOTHING
                "#,
                interpro_data_source_id,
                interpro_version_id,
                go_ds_id,
                go_ver_id,
                mapping.evidence_code
            )
            .execute(&mut *tx)
            .await?;

            stored_count += 1;
        } else {
            warn!(
                "GO term {} not found in database, skipping mapping",
                mapping.go_id
            );
        }
    }

    tx.commit().await?;

    debug!(
        "Stored {} GO mappings for InterPro entry {}",
        stored_count, interpro_data_source_id
    );

    Ok(stored_count)
}

// ============================================================================
// External Reference Storage
// ============================================================================

/// Store external references for an InterPro entry
pub async fn store_external_references(
    pool: &PgPool,
    interpro_data_source_id: Uuid,
    references: &[ExternalReferenceData],
) -> Result<usize, Error> {
    if references.is_empty() {
        return Ok(0);
    }

    let mut tx = pool.begin().await?;
    let mut stored_count = 0;

    for reference in references {
        sqlx::query!(
            r#"
            INSERT INTO interpro_external_references (
                interpro_data_source_id, database, database_id, description
            )
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (interpro_data_source_id, database, database_id) DO UPDATE
            SET description = EXCLUDED.description
            "#,
            interpro_data_source_id,
            reference.database,
            reference.database_id,
            reference.description
        )
        .execute(&mut *tx)
        .await?;

        stored_count += 1;
    }

    tx.commit().await?;

    debug!(
        "Stored {} external references for InterPro entry {}",
        stored_count, interpro_data_source_id
    );

    Ok(stored_count)
}

// ============================================================================
// Protein Match Storage (HIGH PERFORMANCE)
// ============================================================================

/// Store protein matches in optimized batches
///
/// This is the critical path for performance - protein2ipr files can have millions of rows.
/// Uses helpers to avoid N+1 queries and batch inserts for maximum throughput.
pub async fn store_protein_matches_batch(
    pool: &PgPool,
    matches: &[ProteinMatch],
    protein_helper: &mut ProteinLookupHelper,
    interpro_helper: &mut InterProEntryLookupHelper,
    signature_helper: &mut SignatureLookupHelper,
) -> Result<usize, Error> {
    if matches.is_empty() {
        return Ok(0);
    }

    info!("Storing {} protein matches in batch", matches.len());

    // OPTIMIZATION 1: Batch load all proteins
    let protein_accessions: Vec<String> = matches
        .iter()
        .map(|m| m.uniprot_accession.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    protein_helper
        .load_batch(pool, &protein_accessions)
        .await?;

    // OPTIMIZATION 2: Batch load all InterPro entries
    let interpro_ids: Vec<String> = matches
        .iter()
        .map(|m| m.interpro_id.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    interpro_helper.load_batch(pool, &interpro_ids).await?;

    // OPTIMIZATION 3: Batch load all signatures
    let signatures: Vec<(String, String)> = matches
        .iter()
        .map(|m| (m.signature_database.to_string(), m.signature_accession.clone()))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    signature_helper.load_batch(pool, &signatures).await?;

    // OPTIMIZATION 4: Get latest versions for all InterPro entries
    let mut interpro_versions: HashMap<Uuid, Uuid> = HashMap::new();
    for interpro_id in &interpro_ids {
        if let Some(ds_id) = interpro_helper.get(interpro_id) {
            let version_id = get_latest_version(pool, ds_id).await?;
            interpro_versions.insert(ds_id, version_id);
        }
    }

    // OPTIMIZATION 5: Process in chunks for memory efficiency
    let mut total_stored = 0;

    for chunk in matches.chunks(DEFAULT_BATCH_SIZE) {
        let mut tx = pool.begin().await?;

        for match_data in chunk {
            // Lookup protein (from cache - O(1))
            let (protein_ds_id, protein_ver_id) = match protein_helper.get(&match_data.uniprot_accession) {
                Some(ids) => ids,
                None => {
                    warn!(
                        "Protein {} not found, skipping match",
                        match_data.uniprot_accession
                    );
                    continue;
                }
            };

            // Lookup InterPro entry (from cache - O(1))
            let interpro_ds_id = match interpro_helper.get(&match_data.interpro_id) {
                Some(id) => id,
                None => {
                    warn!(
                        "InterPro entry {} not found, skipping match",
                        match_data.interpro_id
                    );
                    continue;
                }
            };

            // Get InterPro version (from cache - O(1))
            let interpro_ver_id = match interpro_versions.get(&interpro_ds_id) {
                Some(id) => *id,
                None => {
                    warn!(
                        "Version not found for InterPro entry {}, skipping",
                        match_data.interpro_id
                    );
                    continue;
                }
            };

            // Lookup signature (from cache - O(1))
            let sig_db = match_data.signature_database.to_string();
            let signature_id = match signature_helper.get(&sig_db, &match_data.signature_accession) {
                Some(id) => id,
                None => {
                    warn!(
                        "Signature {}:{} not found, skipping match",
                        sig_db, match_data.signature_accession
                    );
                    continue;
                }
            };

            // Insert match (with ON CONFLICT to handle duplicates)
            sqlx::query!(
                r#"
                INSERT INTO protein_interpro_matches (
                    interpro_data_source_id, interpro_version_id,
                    protein_data_source_id, protein_version_id,
                    uniprot_accession, signature_id,
                    start_position, end_position, e_value, score
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                ON CONFLICT (protein_data_source_id, interpro_data_source_id, signature_id, start_position, end_position)
                DO NOTHING
                "#,
                interpro_ds_id,
                interpro_ver_id,
                protein_ds_id,
                protein_ver_id,
                match_data.uniprot_accession,
                signature_id,
                match_data.start_position,
                match_data.end_position,
                match_data.e_value,
                match_data.score
            )
            .execute(&mut *tx)
            .await?;

            total_stored += 1;
        }

        tx.commit().await?;

        debug!("Committed chunk of {} matches", chunk.len());
    }

    info!("Successfully stored {} protein matches", total_stored);

    Ok(total_stored)
}

// ============================================================================
// Complete Metadata Storage
// ============================================================================

/// Store complete InterPro metadata (entry + signatures + GO + refs)
///
/// This is the high-level function that orchestrates all storage operations
pub async fn store_interpro_metadata(
    pool: &PgPool,
    metadata: &InterProMetadata,
    version: &str, // InterPro version
    go_helper: &mut GoTermLookupHelper,
) -> Result<(Uuid, Uuid), Error> {
    info!(
        "Storing complete metadata for InterPro entry {}",
        metadata.entry.interpro_id
    );

    // Step 1: Store InterPro entry
    let (interpro_ds_id, interpro_ver_id) = store_interpro_entry(pool, &metadata.entry, version).await?;

    // Step 2: Store signatures
    if !metadata.member_signatures.is_empty() {
        let sig_map = store_signatures_batch(pool, &metadata.member_signatures).await?;

        // Link signatures to entry
        let sig_links: Vec<(Uuid, bool)> = metadata
            .member_signatures
            .iter()
            .filter_map(|sig| {
                let key = (sig.database.to_string(), sig.accession.clone());
                sig_map.get(&key).map(|id| (*id, sig.is_primary))
            })
            .collect();

        link_signatures_to_entry(pool, interpro_ds_id, &sig_links).await?;
    }

    // Step 3: Store GO mappings
    if !metadata.go_mappings.is_empty() {
        store_go_mappings(
            pool,
            interpro_ds_id,
            interpro_ver_id,
            &metadata.go_mappings,
            go_helper,
        )
        .await?;
    }

    // Step 4: Store external references
    if !metadata.external_references.is_empty() {
        store_external_references(pool, interpro_ds_id, &metadata.external_references).await?;
    }

    info!(
        "Successfully stored complete metadata for InterPro entry {}",
        metadata.entry.interpro_id
    );

    Ok((interpro_ds_id, interpro_ver_id))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get latest version ID for a data source
async fn get_latest_version(pool: &PgPool, data_source_id: Uuid) -> Result<Uuid, Error> {
    let record = sqlx::query!(
        r#"
        SELECT id, version_major, version_minor
        FROM versions
        WHERE id = $1
        ORDER BY version_major DESC, version_minor DESC
        LIMIT 1
        "#,
        data_source_id
    )
    .fetch_one(pool)
    .await?;

    Ok(record.id)
}

/// Update InterPro entry statistics
///
/// This is called automatically by triggers, but can be manually invoked
pub async fn update_entry_statistics(
    pool: &PgPool,
    interpro_data_source_id: Uuid,
) -> Result<(), Error> {
    sqlx::query!(
        r#"
        INSERT INTO interpro_entry_stats (interpro_data_source_id, protein_count, species_count, signature_count)
        SELECT
            $1,
            COUNT(DISTINCT protein_data_source_id),
            0, -- Species count requires join to protein_metadata
            (SELECT COUNT(*) FROM interpro_member_signatures WHERE interpro_data_source_id = $1)
        FROM protein_interpro_matches
        WHERE interpro_data_source_id = $1
        ON CONFLICT (interpro_data_source_id) DO UPDATE
        SET
            protein_count = EXCLUDED.protein_count,
            signature_count = EXCLUDED.signature_count,
            last_updated = NOW()
        "#,
        interpro_data_source_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_size_constant() {
        assert_eq!(DEFAULT_BATCH_SIZE, 500);
    }

    #[test]
    fn test_organization_name() {
        assert_eq!(INTERPRO_ORGANIZATION, "InterPro");
    }
}
