use mediator::Request;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSpec {
    pub organization: String,
    pub name: String,
    pub version: String,
    pub format: String,
}

impl SourceSpec {
    /// Parse a source specification in the format: registry:identifier-format@version
    ///
    /// Examples:
    /// - "uniprot:P01308-fasta@1.0"
    /// - "ncbi:GRCh38-xml@2.0"
    ///
    /// The format is always the last segment after the last hyphen in the identifier.
    pub fn parse(spec: &str) -> Result<Self, String> {
        let parts: Vec<&str> = spec.split('@').collect();
        if parts.len() != 2 {
            return Err(format!(
                "Invalid spec format: '{}'. Expected 'registry:identifier-format@version'",
                spec
            ));
        }

        let registry_identifier = parts[0];
        let version = parts[1].to_string();

        let prefix_parts: Vec<&str> = registry_identifier.split(':').collect();
        if prefix_parts.len() != 2 {
            return Err(format!(
                "Invalid spec format: '{}'. Expected 'registry:identifier-format@version'",
                spec
            ));
        }

        let organization = prefix_parts[0].to_string();
        let identifier_with_format = prefix_parts[1];

        // Extract format from identifier (last segment after '-')
        let format_parts: Vec<&str> = identifier_with_format.split('-').collect();
        if format_parts.len() < 2 {
            return Err(format!(
                "Invalid spec format: '{}'. Expected 'registry:identifier-format@version' with format suffix",
                spec
            ));
        }

        // format_parts.len() >= 2 is guaranteed by the check above
        let format = format_parts.last()
            .ok_or_else(|| format!(
                "Invalid spec format: '{}'. Expected 'registry:identifier-format@version' with format suffix",
                spec
            ))?
            .to_string();
        let name = format_parts[..format_parts.len() - 1].join("-");

        Ok(Self {
            organization,
            name,
            version,
            format,
        })
    }

    pub fn to_key(&self) -> String {
        format!("{}:{}-{}@{}", self.organization, self.name, self.format, self.version)
    }

    pub fn to_source(&self) -> String {
        format!("{}:{}@{}", self.organization, self.name, self.version)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub organization: String,
    pub name: String,
    pub version: String,
}

impl ToolSpec {
    pub fn parse(spec: &str) -> Result<Self, String> {
        let parts: Vec<&str> = spec.split('@').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid tool spec: '{}'. Expected 'org:name@version'", spec));
        }

        let prefix = parts[0];
        let version = parts[1];

        let prefix_parts: Vec<&str> = prefix.split(':').collect();
        if prefix_parts.len() != 2 {
            return Err(format!("Invalid tool spec: '{}'. Expected 'org:name@version'", spec));
        }

        Ok(Self {
            organization: prefix_parts[0].to_string(),
            name: prefix_parts[1].to_string(),
            version: version.to_string(),
        })
    }

    pub fn to_key(&self) -> String {
        format!("{}:{}@{}", self.organization, self.name, self.version)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveManifestQuery {
    pub sources: Vec<String>,
    #[serde(default)]
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveManifestResponse {
    pub sources: HashMap<String, ResolvedSource>,
    pub tools: HashMap<String, ResolvedTool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedSource {
    pub resolved: String,
    pub format: String,
    pub checksum: String,
    pub size: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_version: Option<String>,
    pub has_dependencies: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependency_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<DependencyInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedTool {
    pub resolved: String,
    pub checksum: String,
    pub size: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub source: String,
    pub format: String,
    pub checksum: String,
    pub size: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveManifestError {
    #[error("Invalid source specification: '{0}'. Expected format: 'registry:identifier-format@version' (e.g., 'uniprot:P01308-fasta@1.0').")]
    InvalidSourceSpec(String),
    #[error("Invalid tool specification: '{0}'. Expected format: 'registry:name@version' (e.g., 'ncbi:blast@2.14.0').")]
    InvalidToolSpec(String),
    #[error("Source '{0}' not found in registry. Check the organization name and data source identifier.")]
    SourceNotFound(String),
    #[error("Tool '{0}' not found in registry. Check the organization name and tool identifier.")]
    ToolNotFound(String),
    #[error(
        "Version '{0}' not available. Use the API to list available versions for this source."
    )]
    VersionNotFound(String),
    #[error("Format '{0}' not available for this data source. Check available formats with the data source details endpoint.")]
    FormatNotAvailable(String),
    #[error("Dependency conflict: {0}. Two sources require incompatible versions of the same dependency.")]
    DependencyConflict(String),
    #[error("Circular dependency detected involving '{0}'. Dependencies must not form a cycle.")]
    CircularDependency(String),
    #[error("Failed to resolve manifest: {0}")]
    Database(#[from] sqlx::Error),
}

impl Request<Result<ResolveManifestResponse, ResolveManifestError>> for ResolveManifestQuery {}

impl crate::cqrs::middleware::Query for ResolveManifestQuery {}

impl ResolveManifestQuery {
    pub fn validate(&self) -> Result<(), ResolveManifestError> {
        if self.sources.is_empty() && self.tools.is_empty() {
            return Err(ResolveManifestError::InvalidSourceSpec(
                "At least one source or tool is required".to_string(),
            ));
        }
        Ok(())
    }
}

#[tracing::instrument(skip(pool))]
pub async fn handle(
    pool: PgPool,
    query: ResolveManifestQuery,
) -> Result<ResolveManifestResponse, ResolveManifestError> {
    query.validate()?;

    let mut resolved_sources = HashMap::new();
    let mut resolved_tools = HashMap::new();

    for source_spec_str in &query.sources {
        let spec =
            SourceSpec::parse(source_spec_str).map_err(ResolveManifestError::InvalidSourceSpec)?;

        let resolved = resolve_source(&pool, &spec).await?;
        resolved_sources.insert(source_spec_str.clone(), resolved);
    }

    for tool_spec_str in &query.tools {
        let spec = ToolSpec::parse(tool_spec_str).map_err(ResolveManifestError::InvalidToolSpec)?;

        let resolved = resolve_tool(&pool, &spec).await?;
        resolved_tools.insert(tool_spec_str.clone(), resolved);
    }

    detect_conflicts(&resolved_sources)?;

    Ok(ResolveManifestResponse {
        sources: resolved_sources,
        tools: resolved_tools,
    })
}

async fn resolve_source(
    pool: &PgPool,
    spec: &SourceSpec,
) -> Result<ResolvedSource, ResolveManifestError> {
    let entry = sqlx::query!(
        r#"
        SELECT re.id, re.slug
        FROM registry_entries re
        JOIN organizations o ON o.id = re.organization_id
        WHERE LOWER(o.slug) = LOWER($1) AND LOWER(re.slug) = LOWER($2) AND re.entry_type = 'data_source'
        "#,
        spec.organization,
        spec.name
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        ResolveManifestError::SourceNotFound(format!("{}:{}", spec.organization, spec.name))
    })?;

    let version = sqlx::query!(
        r#"
        SELECT id, version, external_version, dependency_count
        FROM versions
        WHERE entry_id = $1 AND version = $2
        "#,
        entry.id,
        spec.version
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        ResolveManifestError::VersionNotFound(format!(
            "{}:{}@{}",
            spec.organization, spec.name, spec.version
        ))
    })?;

    let file = sqlx::query!(
        r#"
        SELECT checksum, size_bytes
        FROM version_files
        WHERE version_id = $1 AND format = $2
        "#,
        version.id,
        spec.format
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        ResolveManifestError::FormatNotAvailable(format!(
            "Format '{}' not available for {}:{}@{}",
            spec.format, spec.organization, spec.name, spec.version
        ))
    })?;

    let has_dependencies = version.dependency_count.unwrap_or(0) > 0;

    let dependencies = if has_dependencies {
        Some(fetch_dependencies(pool, version.id).await?)
    } else {
        None
    };

    Ok(ResolvedSource {
        resolved: spec.to_source(),
        format: spec.format.clone(),
        checksum: file.checksum,
        size: file.size_bytes,
        external_version: version.external_version,
        has_dependencies,
        dependency_count: version.dependency_count,
        dependencies,
    })
}

async fn fetch_dependencies(
    pool: &PgPool,
    version_id: Uuid,
) -> Result<Vec<DependencyInfo>, ResolveManifestError> {
    let deps = sqlx::query!(
        r#"
        SELECT
            o.slug as org_slug,
            re.slug as entry_slug,
            d.depends_on_version,
            vf.format,
            vf.checksum,
            vf.size_bytes
        FROM dependencies d
        JOIN registry_entries re ON re.id = d.depends_on_entry_id
        JOIN organizations o ON o.id = re.organization_id
        JOIN versions v ON v.entry_id = re.id AND v.version = d.depends_on_version
        JOIN version_files vf ON vf.version_id = v.id
        WHERE d.version_id = $1
        ORDER BY re.slug
        LIMIT 100
        "#,
        version_id
    )
    .fetch_all(pool)
    .await?;

    Ok(deps
        .into_iter()
        .map(|dep| DependencyInfo {
            source: format!("{}:{}@{}", dep.org_slug, dep.entry_slug, dep.depends_on_version),
            format: dep.format,
            checksum: dep.checksum,
            size: dep.size_bytes,
        })
        .collect())
}

async fn resolve_tool(
    pool: &PgPool,
    spec: &ToolSpec,
) -> Result<ResolvedTool, ResolveManifestError> {
    let entry = sqlx::query!(
        r#"
        SELECT re.id
        FROM registry_entries re
        JOIN organizations o ON o.id = re.organization_id
        WHERE LOWER(o.slug) = LOWER($1) AND LOWER(re.slug) = LOWER($2) AND re.entry_type = 'tool'
        "#,
        spec.organization,
        spec.name
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        ResolveManifestError::ToolNotFound(format!("{}:{}", spec.organization, spec.name))
    })?;

    let version = sqlx::query!(
        r#"
        SELECT id, external_version, size_bytes
        FROM versions
        WHERE entry_id = $1 AND version = $2
        "#,
        entry.id,
        spec.version
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        ResolveManifestError::VersionNotFound(format!(
            "Tool {}:{}@{}",
            spec.organization, spec.name, spec.version
        ))
    })?;

    let file = sqlx::query!(
        r#"
        SELECT checksum
        FROM version_files
        WHERE version_id = $1
        ORDER BY created_at
        LIMIT 1
        "#,
        version.id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        ResolveManifestError::ToolNotFound(format!(
            "No files found for tool {}:{}@{}",
            spec.organization, spec.name, spec.version
        ))
    })?;

    Ok(ResolvedTool {
        resolved: spec.to_key(),
        checksum: file.checksum,
        size: version.size_bytes.unwrap_or(0),
        external_version: version.external_version,
    })
}

fn detect_conflicts(
    resolved_sources: &HashMap<String, ResolvedSource>,
) -> Result<(), ResolveManifestError> {
    let mut version_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut visited: HashSet<String> = HashSet::new();

    for source in resolved_sources.values() {
        let base_key = source.resolved.clone();

        if visited.contains(&base_key) {
            continue;
        }
        visited.insert(base_key.clone());

        // base_key is in format "org:name@version", split by '@' to get base and version
        let parts: Vec<&str> = base_key.split('@').collect();
        if parts.len() == 2 {
            version_map
                .entry(parts[0].to_string())
                .or_default()
                .insert(parts[1].to_string());
        }

        if let Some(deps) = &source.dependencies {
            check_dependencies(deps, &mut version_map)?;
        }
    }

    for (base, versions) in version_map {
        if versions.len() > 1 {
            return Err(ResolveManifestError::DependencyConflict(format!(
                "Multiple versions of '{}': {:?}",
                base,
                versions.iter().collect::<Vec<_>>()
            )));
        }
    }

    Ok(())
}

fn check_dependencies(
    deps: &[DependencyInfo],
    version_map: &mut HashMap<String, HashSet<String>>,
) -> Result<(), ResolveManifestError> {
    for dep in deps {
        let parts: Vec<&str> = dep.source.split('@').collect();
        if parts.len() == 2 {
            let base = parts[0].to_string();
            let version = parts[1].to_string();

            version_map
                .entry(base.clone())
                .or_default()
                .insert(version.clone());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_source_spec_valid() {
        let spec = SourceSpec::parse("uniprot:P01308-fasta@1.0").unwrap();
        assert_eq!(spec.organization, "uniprot");
        assert_eq!(spec.name, "P01308");
        assert_eq!(spec.version, "1.0");
        assert_eq!(spec.format, "fasta");
    }

    #[test]
    fn test_parse_source_spec_invalid() {
        assert!(SourceSpec::parse("invalid").is_err());
        assert!(SourceSpec::parse("uniprot:P01308").is_err());
        assert!(SourceSpec::parse("uniprot-fasta@1.0").is_err());
        assert!(SourceSpec::parse("uniprot:P01308@1.0").is_err());
    }

    #[test]
    fn test_parse_tool_spec_valid() {
        let spec = ToolSpec::parse("ncbi:blast@2.14.0").unwrap();
        assert_eq!(spec.organization, "ncbi");
        assert_eq!(spec.name, "blast");
        assert_eq!(spec.version, "2.14.0");
    }

    #[test]
    fn test_parse_tool_spec_invalid() {
        assert!(ToolSpec::parse("invalid").is_err());
        assert!(ToolSpec::parse("ncbi:blast").is_err());
        assert!(ToolSpec::parse("blast@2.14.0").is_err());
    }

    #[test]
    fn test_source_spec_to_key() {
        let spec = SourceSpec {
            organization: "uniprot".to_string(),
            name: "P01308".to_string(),
            version: "1.0".to_string(),
            format: "fasta".to_string(),
        };
        assert_eq!(spec.to_key(), "uniprot:P01308-fasta@1.0");
        assert_eq!(spec.to_source(), "uniprot:P01308@1.0");
    }

    #[test]
    fn test_tool_spec_to_key() {
        let spec = ToolSpec {
            organization: "ncbi".to_string(),
            name: "blast".to_string(),
            version: "2.14.0".to_string(),
        };
        assert_eq!(spec.to_key(), "ncbi:blast@2.14.0");
    }

    #[test]
    fn test_validation_empty_sources_and_tools() {
        let query = ResolveManifestQuery {
            sources: vec![],
            tools: vec![],
        };
        assert!(query.validate().is_err());
    }

    #[test]
    fn test_validation_with_sources() {
        let query = ResolveManifestQuery {
            sources: vec!["uniprot:P01308-fasta@1.0".to_string()],
            tools: vec![],
        };
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validation_with_tools() {
        let query = ResolveManifestQuery {
            sources: vec![],
            tools: vec!["ncbi:blast@2.14.0".to_string()],
        };
        assert!(query.validate().is_ok());
    }

    #[sqlx::test]
    async fn test_handle_resolve_source(pool: PgPool) -> sqlx::Result<()> {
        let org_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, slug, name, is_system)
            VALUES ($1, $2, $3, $4)
            "#,
            org_id,
            "uniprot",
            "UniProt",
            true
        )
        .execute(&pool)
        .await?;

        let entry_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO registry_entries (id, organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            entry_id,
            org_id,
            "P01308",
            "Insulin",
            "data_source"
        )
        .execute(&pool)
        .await?;

        let version_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO versions (id, entry_id, version, external_version, dependency_count)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            version_id,
            entry_id,
            "1.0",
            Some("2025_01"),
            0
        )
        .execute(&pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO version_files (version_id, format, s3_key, checksum, size_bytes)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            version_id,
            "fasta",
            "test/P01308.fasta",
            "abc123",
            1024i64
        )
        .execute(&pool)
        .await?;

        let query = ResolveManifestQuery {
            sources: vec!["uniprot:P01308-fasta@1.0".to_string()],
            tools: vec![],
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.sources.len(), 1);
        let source = response.sources.get("uniprot:P01308-fasta@1.0").unwrap();
        assert_eq!(source.resolved, "uniprot:P01308@1.0");
        assert_eq!(source.format, "fasta");
        assert_eq!(source.checksum, "abc123");
        assert_eq!(source.size, 1024);
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_source_not_found(pool: PgPool) -> sqlx::Result<()> {
        let query = ResolveManifestQuery {
            sources: vec!["nonexistent:source-fasta@1.0".to_string()],
            tools: vec![],
        };

        let result = handle(pool.clone(), query).await;
        assert!(matches!(result, Err(ResolveManifestError::SourceNotFound(_))));
        Ok(())
    }

    #[sqlx::test]
    async fn test_handle_resolve_with_dependencies(pool: PgPool) -> sqlx::Result<()> {
        let org_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO organizations (id, slug, name, is_system)
            VALUES ($1, $2, $3, $4)
            "#,
            org_id,
            "uniprot",
            "UniProt",
            true
        )
        .execute(&pool)
        .await?;

        let entry_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO registry_entries (id, organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            entry_id,
            org_id,
            "all",
            "All Proteins",
            "data_source"
        )
        .execute(&pool)
        .await?;

        let dep_entry_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO registry_entries (id, organization_id, slug, name, entry_type)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            dep_entry_id,
            org_id,
            "P01308",
            "Insulin",
            "data_source"
        )
        .execute(&pool)
        .await?;

        let version_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO versions (id, entry_id, version, dependency_count)
            VALUES ($1, $2, $3, $4)
            "#,
            version_id,
            entry_id,
            "1.0",
            1
        )
        .execute(&pool)
        .await?;

        let dep_version_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO versions (id, entry_id, version)
            VALUES ($1, $2, $3)
            "#,
            dep_version_id,
            dep_entry_id,
            "1.0"
        )
        .execute(&pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO version_files (version_id, format, s3_key, checksum, size_bytes)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            version_id,
            "fasta",
            "test/all.fasta",
            "def456",
            2048i64
        )
        .execute(&pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO version_files (version_id, format, s3_key, checksum, size_bytes)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            dep_version_id,
            "fasta",
            "test/P01308.fasta",
            "abc123",
            1024i64
        )
        .execute(&pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO dependencies (version_id, depends_on_entry_id, depends_on_version)
            VALUES ($1, $2, $3)
            "#,
            version_id,
            dep_entry_id,
            "1.0"
        )
        .execute(&pool)
        .await?;

        let query = ResolveManifestQuery {
            sources: vec!["uniprot:all-fasta@1.0".to_string()],
            tools: vec![],
        };

        let result = handle(pool.clone(), query).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.sources.len(), 1);
        let source = response.sources.get("uniprot:all-fasta@1.0").unwrap();
        assert!(source.has_dependencies);
        assert_eq!(source.dependency_count, Some(1));
        assert!(source.dependencies.is_some());
        let deps = source.dependencies.as_ref().unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].source, "uniprot:P01308@1.0");
        Ok(())
    }
}
