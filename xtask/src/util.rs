use crate::utils::*;
/// Utility commands
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub enum UtilCommand {
    /// Show environment info
    Info,
    /// Check database connection
    CheckDb,
    /// Show logs for all services
    Logs,
    /// Follow backend logs
    LogsBackend,
    /// Follow frontend logs
    LogsFrontend,
    /// Health check all services
    Health,
    /// Show current version
    Version,
    /// View recent audit logs
    AuditLogs {
        #[arg(default_value = "50")]
        limit: String,
    },
    /// Search audit logs by action
    AuditSearch { term: String },
    /// View audit logs for a specific resource type
    AuditByResource { resource_type: String },
    /// View audit logs for a specific user
    AuditByUser { user_id: String },
    /// View audit trail for a specific resource
    AuditTrail {
        resource_type: String,
        resource_id: String,
    },
    /// Export audit logs to JSON
    AuditExport {
        #[arg(default_value = "audit_logs.json")]
        output: String,
    },
    /// Show audit statistics
    AuditStats,
}

pub fn handle(cmd: UtilCommand) -> Result<()> {
    match cmd {
        UtilCommand::Info => info_cmd(),
        UtilCommand::CheckDb => check_db(),
        UtilCommand::Logs => logs(),
        UtilCommand::LogsBackend => logs_backend(),
        UtilCommand::LogsFrontend => logs_frontend(),
        UtilCommand::Health => health(),
        UtilCommand::Version => version(),
        UtilCommand::AuditLogs { limit } => audit_logs(&limit),
        UtilCommand::AuditSearch { term } => audit_search(&term),
        UtilCommand::AuditByResource { resource_type } => audit_by_resource(&resource_type),
        UtilCommand::AuditByUser { user_id } => audit_by_user(&user_id),
        UtilCommand::AuditTrail {
            resource_type,
            resource_id,
        } => audit_trail(&resource_type, &resource_id),
        UtilCommand::AuditExport { output } => audit_export(&output),
        UtilCommand::AuditStats => audit_stats(),
    }
}

fn info_cmd() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "📊 BDP Environment Info"
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
$rust = rustc --version; Write-Host "Rust:        $rust"
$cargo = cargo --version; Write-Host "Cargo:       $cargo"
$node = node --version; Write-Host "Node:        $node"
$npm = npm --version; Write-Host "NPM:         $npm"
$docker = docker --version; Write-Host "Docker:      $docker"
try { $sqlx = sqlx --version; Write-Host "SQLx:        $sqlx" } catch { Write-Host "SQLx:        Not installed" }
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
Write-Host "Backend URL: http://localhost:8000"
Write-Host "Frontend URL: http://localhost:3000"
Write-Host "MinIO Console: http://localhost:9001"
Write-Host "Database: postgresql://localhost:5432/bdp"
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
"#,
            "Show environment info",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "📊 BDP Environment Info"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Rust:        $(rustc --version)"
echo "Cargo:       $(cargo --version)"
echo "Node:        $(node --version)"
echo "NPM:         $(npm --version)"
echo "Docker:      $(docker --version)"
echo "SQLx:        $(sqlx --version 2>&1 || echo 'Not installed')"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Backend URL: http://localhost:8000"
echo "Frontend URL: http://localhost:3000"
echo "MinIO Console: http://localhost:9001"
echo "Database: postgresql://localhost:5432/bdp"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
"#,
            "Show environment info",
        )
    }
}

fn check_db() -> Result<()> {
    info("🔍 Checking database connection...");
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_password@localhost:5432/bdp".to_string());

    match run("psql", &[&database_url, "-c", "SELECT version();"], "Check database") {
        Ok(_) => {
            success("Database connected");
            Ok(())
        },
        Err(_) => {
            error("Database connection failed");
            anyhow::bail!("Database connection failed");
        },
    }
}

fn logs() -> Result<()> {
    run_streaming("docker", &["compose", "logs", "-f"], "Show logs")
}

fn logs_backend() -> Result<()> {
    run_streaming("docker", &["compose", "logs", "-f", "bdp-server"], "Backend logs")
}

fn logs_frontend() -> Result<()> {
    run_streaming("docker", &["compose", "logs", "-f", "web"], "Frontend logs")
}

fn health() -> Result<()> {
    info("🏥 Checking service health...");

    // Check backend
    if run("curl", &["-s", "http://localhost:8000/health"], "Check backend").is_ok() {
        success("Backend healthy");
    } else {
        warning("Backend down");
    }

    // Check frontend
    if run("curl", &["-s", "http://localhost:3000"], "Check frontend").is_ok() {
        success("Frontend healthy");
    } else {
        warning("Frontend down");
    }

    // Check MinIO
    if run("curl", &["-s", "http://localhost:9000/minio/health/live"], "Check MinIO").is_ok() {
        success("MinIO healthy");
    } else {
        warning("MinIO down");
    }

    Ok(())
}

fn version() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "📦 BDP Version Information"
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
$metadata = cargo metadata --format-version 1 --no-deps | ConvertFrom-Json
$rustVersion = $metadata.packages | Where-Object { $_.name -eq "bdp-cli" } | Select-Object -ExpandProperty version
Write-Host "Rust:    v$rustVersion"
cd web; $nodeVersion = (Get-Content package.json | ConvertFrom-Json).version
Write-Host "Node:    v$nodeVersion"
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
"#,
            "Show version",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "📦 BDP Version Information"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.name=="bdp-cli") | "Rust:    v" + .version'
cd web && node -p "'Node:    v' + require('./package.json').version"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
"#,
            "Show version",
        )
    }
}

fn audit_logs(limit: &str) -> Result<()> {
    info(&format!("📋 Viewing recent audit logs (limit: {})...", limit));
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_password@localhost:5432/bdp".to_string());
    let query = format!("SELECT id, timestamp, action, resource_type, resource_id, user_id FROM audit_log ORDER BY timestamp DESC LIMIT {};", limit);
    run_streaming("psql", &[&database_url, "-c", &query], "View audit logs")
}

fn audit_search(term: &str) -> Result<()> {
    info(&format!("🔍 Searching audit logs for: {}", term));
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_password@localhost:5432/bdp".to_string());
    let query = format!("SELECT id, timestamp, action, resource_type, resource_id, user_id, changes FROM audit_log WHERE action ILIKE '%{}%' OR resource_type ILIKE '%{}%' OR changes::text ILIKE '%{}%' ORDER BY timestamp DESC LIMIT 50;", term, term, term);
    run_streaming("psql", &[&database_url, "-c", &query], "Search audit logs")
}

fn audit_by_resource(resource_type: &str) -> Result<()> {
    info(&format!("📋 Viewing audit logs for resource type: {}", resource_type));
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_password@localhost:5432/bdp".to_string());
    let query = format!("SELECT id, timestamp, action, resource_type, resource_id, user_id, changes FROM audit_log WHERE resource_type = '{}' ORDER BY timestamp DESC LIMIT 50;", resource_type);
    run_streaming("psql", &[&database_url, "-c", &query], "View audit logs by resource")
}

fn audit_by_user(user_id: &str) -> Result<()> {
    info(&format!("📋 Viewing audit logs for user: {}", user_id));
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_password@localhost:5432/bdp".to_string());
    let query = format!("SELECT id, timestamp, action, resource_type, resource_id, changes FROM audit_log WHERE user_id = '{}'::uuid ORDER BY timestamp DESC LIMIT 50;", user_id);
    run_streaming("psql", &[&database_url, "-c", &query], "View audit logs by user")
}

fn audit_trail(resource_type: &str, resource_id: &str) -> Result<()> {
    info(&format!("📋 Viewing audit trail for {} {}", resource_type, resource_id));
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_password@localhost:5432/bdp".to_string());
    let query = format!("SELECT id, timestamp, action, user_id, changes, metadata FROM audit_log WHERE resource_type = '{}' AND resource_id = '{}'::uuid ORDER BY timestamp ASC;", resource_type, resource_id);
    run_streaming("psql", &[&database_url, "-c", &query], "View audit trail")
}

fn audit_export(output: &str) -> Result<()> {
    info(&format!("💾 Exporting audit logs to {}...", output));
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_password@localhost:5432/bdp".to_string());

    #[cfg(target_os = "windows")]
    {
        let query = "SELECT row_to_json(t) FROM (SELECT id, timestamp, action, resource_type, resource_id, user_id, changes, metadata, ip_address FROM audit_log ORDER BY timestamp DESC LIMIT 1000) t;";
        let output_str = run_output("psql", &[&database_url, "-t", "-A", "-F,", "-c", query])?;
        std::fs::write(output, output_str)?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            &format!(
                r#"
psql {} -t -A -F"," -c "SELECT row_to_json(t) FROM (SELECT id, timestamp, action, resource_type, resource_id, user_id, changes, metadata, ip_address FROM audit_log ORDER BY timestamp DESC LIMIT 1000) t;" > {}
"#,
                database_url, output
            ),
            "Export audit logs",
        )?;
    }

    success(&format!("Exported to {}", output));
    Ok(())
}

fn audit_stats() -> Result<()> {
    info("📊 Audit Log Statistics");
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://bdp:bdp_password@localhost:5432/bdp".to_string());

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    run_streaming(
        "psql",
        &[
            &database_url,
            "-c",
            "SELECT action, COUNT(*) as count FROM audit_log GROUP BY action ORDER BY count DESC;",
        ],
        "Actions",
    )?;
    println!();
    run_streaming("psql", &[&database_url, "-c", "SELECT resource_type, COUNT(*) as count FROM audit_log GROUP BY resource_type ORDER BY count DESC;"], "Resource types")?;
    println!();
    run_streaming("psql", &[&database_url, "-c", "SELECT DATE(timestamp) as date, COUNT(*) as count FROM audit_log GROUP BY DATE(timestamp) ORDER BY date DESC LIMIT 7;"], "Last 7 days")?;
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    Ok(())
}
