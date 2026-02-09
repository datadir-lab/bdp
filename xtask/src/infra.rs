use crate::utils::*;
/// Infrastructure (Terraform/OVH Cloud)
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub enum InfraCommand {
    /// Initialize Terraform
    Init,
    /// Preview infrastructure changes
    Plan,
    /// Apply infrastructure changes
    Apply,
    /// Destroy infrastructure (careful!)
    Destroy,
    /// Show infrastructure outputs
    Output,
    /// Generate production .env file from Terraform
    Env,
    /// SSH into production server
    Ssh,
    /// Show infrastructure status
    Status,
}

pub fn handle(cmd: InfraCommand) -> Result<()> {
    match cmd {
        InfraCommand::Init => init(),
        InfraCommand::Plan => plan(),
        InfraCommand::Apply => apply(),
        InfraCommand::Destroy => destroy(),
        InfraCommand::Output => output(),
        InfraCommand::Env => env(),
        InfraCommand::Ssh => ssh(),
        InfraCommand::Status => status(),
    }
}

fn init() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🏗️ Initializing Terraform..."
cd infrastructure; terraform init
Write-Host "✓ Terraform initialized"
"#,
            "Initialize Terraform",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🏗️ Initializing Terraform..."
cd infrastructure && terraform init
echo "✓ Terraform initialized"
"#,
            "Initialize Terraform",
        )
    }
}

fn plan() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🔍 Planning infrastructure changes..."
cd infrastructure; terraform plan
"#,
            "Plan infrastructure",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🔍 Planning infrastructure changes..."
cd infrastructure && terraform plan
"#,
            "Plan infrastructure",
        )
    }
}

fn apply() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🚀 Applying infrastructure..."
cd infrastructure; terraform apply
"#,
            "Apply infrastructure",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🚀 Applying infrastructure..."
cd infrastructure && terraform apply
"#,
            "Apply infrastructure",
        )
    }
}

fn destroy() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "⚠️ Destroying infrastructure..."
cd infrastructure; terraform destroy
"#,
            "Destroy infrastructure",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "⚠️ Destroying infrastructure..."
cd infrastructure && terraform destroy
"#,
            "Destroy infrastructure",
        )
    }
}

fn output() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "📋 Infrastructure outputs:"
cd infrastructure; terraform output
"#,
            "Show infrastructure outputs",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "📋 Infrastructure outputs:"
cd infrastructure && terraform output
"#,
            "Show infrastructure outputs",
        )
    }
}

fn env() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "📝 Generating production .env..."
cd infrastructure; terraform output -raw env_file_content > ../production.env
Write-Host "✓ Generated production.env"
"#,
            "Generate production .env",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "📝 Generating production .env..."
cd infrastructure && terraform output -raw env_file_content > ../production.env
echo "✓ Generated production.env"
"#,
            "Generate production .env",
        )
    }
}

fn ssh() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "🔐 Connecting to production server..."
cd infrastructure; $ip = terraform output -raw instance_ip; ssh ubuntu@$ip
"#,
            "SSH to production",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "🔐 Connecting to production server..."
cd infrastructure && ssh ubuntu@$(terraform output -raw instance_ip)
"#,
            "SSH to production",
        )
    }
}

fn status() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        run_powershell(
            r#"
Write-Host "📊 Infrastructure Status"
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cd infrastructure
try { $ip = terraform output -raw instance_ip 2>$null; Write-Host "Instance IP:  $ip" } catch { Write-Host "Instance:     Not deployed" }
try { $db = terraform output -raw database_host 2>$null; Write-Host "Database:     $db" } catch { Write-Host "Database:     Not deployed" }
try { $s3 = terraform output -raw s3_endpoint 2>$null; Write-Host "S3 Endpoint:  $s3" } catch { Write-Host "S3:           Not deployed" }
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
"#,
            "Show infrastructure status",
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_bash(
            r#"
echo "📊 Infrastructure Status"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cd infrastructure
terraform output -raw instance_ip 2>/dev/null && echo "Instance IP:  $(terraform output -raw instance_ip)" || echo "Instance:     Not deployed"
terraform output -raw database_host 2>/dev/null && echo "Database:     $(terraform output -raw database_host)" || echo "Database:     Not deployed"
terraform output -raw s3_endpoint 2>/dev/null && echo "S3 Endpoint:  $(terraform output -raw s3_endpoint)" || echo "S3:           Not deployed"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
"#,
            "Show infrastructure status",
        )
    }
}
