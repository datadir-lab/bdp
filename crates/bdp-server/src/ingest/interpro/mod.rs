// InterPro Integration Module
//
// This module handles ingestion of InterPro protein family and domain data.
// InterPro integrates multiple protein signature databases (Pfam, SMART, PROSITE, etc.)
// and provides GO term mappings and protein domain annotations.
//
// Architecture:
// - Individual data sources: Each InterPro entry is a separate data source
// - Version-specific FKs: All cross-references use version_id for cascade versioning
// - MAJOR.MINOR versioning: No patch version (e.g., 1.0, 1.1, 2.0)
// - Relational design: NO JSONB for primary data, uses proper foreign keys
//
// Data Flow:
// 1. Download protein2ipr.dat.gz and entry.list from InterPro FTP
// 2. Parse entries and protein matches
// 3. Create/update data sources for each InterPro entry
// 4. Store metadata, signatures, GO mappings, and protein matches
// 5. When UniProt version bumps, cascade to new InterPro versions

pub mod config;
pub mod ftp;
pub mod helpers;
pub mod models;
pub mod parser;
pub mod pipeline;
pub mod storage;
pub mod version_discovery;
