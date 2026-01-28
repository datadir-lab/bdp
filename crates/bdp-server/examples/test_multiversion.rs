// Test Multi-Version InterPro Ingestion

use bdp_server::db::{create_pool, DbConfig};
use bdp_server::ingest::interpro::{
    models::{EntryType, InterProEntry},
    storage::store_interpro_entry,
};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_dev_password@localhost:5432/bdp".to_string());

    let db_config = DbConfig {
        url: database_url,
        max_connections: 10,
        ..Default::default()
    };
    let pool = create_pool(&db_config).await?;

    // Create test entry
    let entry = InterProEntry {
        interpro_id: "IPR_MULTIVERSION".to_string(),
        entry_type: EntryType::Domain,
        name: "Multi-Version Test Entry".to_string(),
        short_name: Some("MVTest".to_string()),
        description: Some("Testing multiple versions".to_string()),
    };

    println!("Testing multi-version support for InterPro");
    println!("Entry: {}", entry.interpro_id);
    println!();

    // Store version 96.0
    println!("Storing version 96.0...");
    let (ds_id_96, ver_id_96) = store_interpro_entry(&pool, &entry, "96.0").await?;
    println!("✓ Version 96.0 stored - DS: {}, Ver: {}", ds_id_96, ver_id_96);

    // Store version 97.0 (same entry, different version)
    println!("Storing version 97.0...");
    let (ds_id_97, ver_id_97) = store_interpro_entry(&pool, &entry, "97.0").await?;
    println!("✓ Version 97.0 stored - DS: {}, Ver: {}", ds_id_97, ver_id_97);

    // Store version 98.0
    println!("Storing version 98.0...");
    let (ds_id_98, ver_id_98) = store_interpro_entry(&pool, &entry, "98.0").await?;
    println!("✓ Version 98.0 stored - DS: {}, Ver: {}", ds_id_98, ver_id_98);

    println!();

    // Verify data source ID is the same across versions
    if ds_id_96 == ds_id_97 && ds_id_97 == ds_id_98 {
        println!("✓ Same data source ID across all versions: {}", ds_id_96);
    } else {
        println!("✗ ERROR: Different data source IDs!");
        println!("  96.0: {}", ds_id_96);
        println!("  97.0: {}", ds_id_97);
        println!("  98.0: {}", ds_id_98);
    }

    // Verify version IDs are different
    if ver_id_96 != ver_id_97 && ver_id_97 != ver_id_98 && ver_id_96 != ver_id_98 {
        println!("✓ Different version IDs for each version");
    } else {
        println!("✗ ERROR: Version IDs should be different!");
    }

    // Query to verify all versions exist
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM versions v
        JOIN data_sources ds ON ds.id = v.entry_id
        JOIN interpro_entry_metadata iem ON iem.data_source_id = ds.id
        WHERE iem.interpro_id = $1
        "#,
    )
    .bind(&entry.interpro_id)
    .fetch_one(&pool)
    .await?;

    println!("✓ Found {} versions in database", count);

    if count == 3 {
        println!();
        println!("✓✓✓ MULTI-VERSION SUPPORT WORKING! ✓✓✓");
    } else {
        println!("✗ Expected 3 versions, found {}", count);
    }

    // Cleanup
    println!();
    println!("Cleaning up test data...");
    sqlx::query!("DELETE FROM interpro_entry_metadata WHERE interpro_id = $1", entry.interpro_id)
        .execute(&pool)
        .await?;
    println!("✓ Cleanup complete");

    Ok(())
}
