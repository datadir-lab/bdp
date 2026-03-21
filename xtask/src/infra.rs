//! Infrastructure operations — Hetzner VPS via Terraform + Dokploy
//!
//! All commands load environment from infrastructure/hetzner/environments/prod/.secrets
//! The .secrets file uses plain `key=value` format (no TF_VAR_ prefix).
//! xtask exports each key both as `key=val` and `TF_VAR_key=val` for Terraform.
//! For GitHub CI, store each key as a secret named TF_VAR_<key>.
use anyhow::{bail, Result};
use clap::Parser;
use std::path::PathBuf;

use crate::utils::*;

const SECRETS_PATH: &str = "infrastructure/hetzner/environments/prod/.secrets";
const TF_DIR: &str = "infrastructure/hetzner/terraform";

#[derive(Debug, Parser)]
pub enum InfraCommand {
    /// One-time setup: generate SSH key + initialize Terraform
    Bootstrap,
    /// Initialize Terraform (after bootstrap)
    Init,
    /// Preview infrastructure changes
    Plan,
    /// Apply infrastructure changes (provisions/updates VPS)
    Apply,
    /// Destroy infrastructure — volume persists (requires confirmation)
    Destroy,
    /// Show Terraform outputs (server IP, URLs, etc.)
    Info,
    /// SSH into production server
    Ssh,
    /// Check live server status (Docker services health)
    Status,
    /// Wait for cloud-init to complete and show credentials
    PostDeploy,
    /// Show all production credentials
    ShowSecrets,
    /// Trigger immediate restic backup
    BackupNow,
    /// List restic snapshots on Storage Box
    BackupList,
    /// Restore from restic backup (interactive)
    Restore,
    /// Tail logs from a service (usage: infra logs [service])
    Logs {
        /// Service name: bdp-server, bdp-web, postgres, minio (default: bdp-server)
        #[arg(default_value = "bdp-server")]
        service: String,
    },
    /// Pull latest Docker images and restart services via Dokploy
    Update,
    /// Validate Terraform configuration
    Validate,
}

pub fn handle(cmd: InfraCommand) -> Result<()> {
    match cmd {
        InfraCommand::Bootstrap => bootstrap(),
        InfraCommand::Init => tf_init(),
        InfraCommand::Plan => tf_plan(),
        InfraCommand::Apply => tf_apply(),
        InfraCommand::Destroy => tf_destroy(),
        InfraCommand::Info => tf_info(),
        InfraCommand::Ssh => ssh_connect(),
        InfraCommand::Status => server_status(),
        InfraCommand::PostDeploy => post_deploy(),
        InfraCommand::ShowSecrets => show_secrets(),
        InfraCommand::BackupNow => backup_now(),
        InfraCommand::BackupList => backup_list(),
        InfraCommand::Restore => backup_restore(),
        InfraCommand::Logs { service } => logs(&service),
        InfraCommand::Update => update_services(),
        InfraCommand::Validate => tf_validate(),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns the path to .secrets, erroring with a helpful message if missing.
fn secrets_path() -> Result<PathBuf> {
    let path = PathBuf::from(SECRETS_PATH);
    if !path.exists() {
        bail!(
            "Secrets file not found: {}\n\
             Copy the example and fill in your values:\n\
             cp {}.example {}",
            SECRETS_PATH,
            SECRETS_PATH,
            SECRETS_PATH
        );
    }
    Ok(path)
}

/// Build the shell preamble that loads .secrets and exports both `key=val`
/// and `TF_VAR_key=val` for each entry. Matches the temnir tf.ps1 pattern.
fn load_env_preamble() -> String {
    format!(
        r#"
set -euo pipefail
# Load .secrets: each key=val line is exported directly AND as TF_VAR_key=val
_bdp_load_secrets() {{
  local _file="$1" _line _key _val
  while IFS= read -r _line || [ -n "$_line" ]; do
    case "$_line" in ''|'#'*) continue ;; esac
    _key="${{_line%%=*}}"
    _val="${{_line#*=}}"
    [ -z "$_key" ] && continue
    export "$_key=$_val" 2>/dev/null || true
    export "TF_VAR_$_key=$_val" 2>/dev/null || true
  done < "$_file"
}}
[ -f "{secrets}" ] && _bdp_load_secrets "{secrets}"
TF_DIR="{tf_dir}"
"#,
        secrets = SECRETS_PATH,
        tf_dir = TF_DIR,
    )
}

/// Get server IP from Terraform outputs
fn get_server_ip() -> Result<String> {
    let preamble = load_env_preamble();
    let script = format!(
        r#"{}
cd "$TF_DIR"
terraform output -raw server_ipv4 2>/dev/null
"#,
        preamble
    );
    let output = {
        #[cfg(not(target_os = "windows"))]
        {
            std::process::Command::new("sh")
                .arg("-c")
                .arg(&script)
                .output()?
        }
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("wsl")
                .args(["bash", "-c", &script])
                .output()?
        }
    };
    if !output.status.success() {
        bail!("Failed to get server IP. Is infrastructure deployed? Run: cargo xtask infra apply");
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn ssh_key_path() -> String {
    // Read ssh_key_path from .secrets (plain key=val format)
    if let Ok(content) = std::fs::read_to_string(SECRETS_PATH) {
        for line in content.lines() {
            let val = line
                .strip_prefix("ssh_key_path=")
                .or_else(|| line.strip_prefix("SSH_KEY_PATH=")); // legacy
            if let Some(val) = val {
                return val
                    .trim()
                    .replace('~', &std::env::var("HOME").unwrap_or_default());
            }
        }
    }
    format!(
        "{}/.ssh/bdp_prod_ed25519",
        std::env::var("HOME").unwrap_or_default()
    )
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn bootstrap() -> Result<()> {
    let preamble = load_env_preamble();
    let script = format!(
        r#"{}
echo "=== BDP Infrastructure Bootstrap ==="
echo ""

# 1. Generate SSH key if it doesn't exist
SSH_KEY="${{ssh_key_path:-$HOME/.ssh/bdp_prod_ed25519}}"
SSH_KEY=$(echo "$SSH_KEY" | sed "s|~|$HOME|")
if [ ! -f "$SSH_KEY" ]; then
  echo "Generating SSH key: $SSH_KEY"
  ssh-keygen -t ed25519 -C "bdp-prod" -f "$SSH_KEY" -N ""
  echo ""
  echo "SSH public key (add to .secrets as: ssh_public_key=<value>):"
  cat "${{SSH_KEY}}.pub"
  echo ""
else
  echo "SSH key already exists: $SSH_KEY"
fi

# 2. Initialize Terraform
echo "Initializing Terraform..."
cd "$TF_DIR"
terraform init

echo ""
echo "Bootstrap complete."
echo ""
echo "Next steps:"
echo "  1. Ensure {secrets} is filled with all required values"
echo "  2. Run: cargo xtask infra plan"
echo "  3. Run: cargo xtask infra apply"
"#,
        preamble,
        secrets = SECRETS_PATH
    );
    #[cfg(not(target_os = "windows"))]
    return run_bash(&script, "Bootstrap infrastructure");
    #[cfg(target_os = "windows")]
    return run_powershell(
        &format!("wsl bash -c '{}'", script.replace('\'', "'\\''")),
        "Bootstrap infrastructure",
    );
}

fn tf_init() -> Result<()> {
    let preamble = load_env_preamble();
    let script = format!(
        r#"{}
echo "Initializing Terraform..."
cd "$TF_DIR"
terraform init
"#,
        preamble
    );
    #[cfg(not(target_os = "windows"))]
    return run_bash(&script, "Terraform init");
    #[cfg(target_os = "windows")]
    return run_powershell(
        &format!("wsl bash -c '{}'", script.replace('\'', "'\\''")),
        "Terraform init",
    );
}

fn tf_plan() -> Result<()> {
    secrets_path()?;
    let preamble = load_env_preamble();
    let script = format!(
        r#"{}
echo "Planning infrastructure changes..."
cd "$TF_DIR"
terraform plan
"#,
        preamble
    );
    #[cfg(not(target_os = "windows"))]
    return run_bash(&script, "Terraform plan");
    #[cfg(target_os = "windows")]
    return run_powershell(
        &format!("wsl bash -c '{}'", script.replace('\'', "'\\''")),
        "Terraform plan",
    );
}

fn tf_apply() -> Result<()> {
    secrets_path()?;
    let preamble = load_env_preamble();
    let script = format!(
        r#"{}
echo "Applying infrastructure..."
cd "$TF_DIR"
terraform apply
echo ""
echo "Done. Run 'cargo xtask infra post-deploy' to wait for cloud-init."
"#,
        preamble
    );
    #[cfg(not(target_os = "windows"))]
    return run_bash(&script, "Terraform apply");
    #[cfg(target_os = "windows")]
    return run_powershell(
        &format!("wsl bash -c '{}'", script.replace('\'', "'\\''")),
        "Terraform apply",
    );
}

fn tf_destroy() -> Result<()> {
    secrets_path()?;
    print!("This will DESTROY the server (volume persists). Type 'yes' to confirm: ");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if input.trim() != "yes" {
        println!("Aborted.");
        return Ok(());
    }
    let preamble = load_env_preamble();
    let script = format!(
        r#"{}
echo "Destroying infrastructure (volume persists)..."
cd "$TF_DIR"
terraform destroy
"#,
        preamble
    );
    #[cfg(not(target_os = "windows"))]
    return run_bash(&script, "Terraform destroy");
    #[cfg(target_os = "windows")]
    return run_powershell(
        &format!("wsl bash -c '{}'", script.replace('\'', "'\\''")),
        "Terraform destroy",
    );
}

fn tf_info() -> Result<()> {
    secrets_path()?;
    let preamble = load_env_preamble();
    let script = format!(
        r#"{}
echo "Infrastructure outputs:"
echo "=================================="
cd "$TF_DIR"
terraform output
"#,
        preamble
    );
    #[cfg(not(target_os = "windows"))]
    return run_bash(&script, "Terraform info");
    #[cfg(target_os = "windows")]
    return run_powershell(
        &format!("wsl bash -c '{}'", script.replace('\'', "'\\''")),
        "Terraform info",
    );
}

fn tf_validate() -> Result<()> {
    let preamble = load_env_preamble();
    let script = format!(
        r#"{}
cd "$TF_DIR"
terraform validate && terraform fmt -check
echo "Terraform configuration is valid."
"#,
        preamble
    );
    #[cfg(not(target_os = "windows"))]
    return run_bash(&script, "Terraform validate");
    #[cfg(target_os = "windows")]
    return run_powershell(
        &format!("wsl bash -c '{}'", script.replace('\'', "'\\''")),
        "Terraform validate",
    );
}

fn ssh_connect() -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    println!("Connecting to root@{ip}...");
    std::process::Command::new("ssh")
        .args([
            "-i",
            &key,
            "-o",
            "StrictHostKeyChecking=accept-new",
            &format!("root@{ip}"),
        ])
        .status()?;
    Ok(())
}

fn server_status() -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    let script = format!(
        r#"
echo "=== BDP Production Status ==="
echo "  Server: {ip}"
echo ""
ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} \
  "docker ps --format 'table {{{{.Names}}}}\t{{{{.Status}}}}\t{{{{.Ports}}}}'"
"#,
        ip = ip,
        key = key
    );
    #[cfg(not(target_os = "windows"))]
    return run_bash(&script, "Server status");
    #[cfg(target_os = "windows")]
    return run_powershell(
        &format!(
            "ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} \
             \"docker ps --format 'table {{{{.Names}}}}\t{{{{.Status}}}}\t{{{{.Ports}}}}'\"",
            ip = ip,
            key = key
        ),
        "Server status",
    );
}

fn post_deploy() -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    let script = format!(
        r#"
echo "=== Waiting for cloud-init to complete ==="
echo "  Server: {ip}"
echo "  This may take 5-10 minutes on first boot..."
echo ""

for i in $(seq 1 60); do
  if ssh -i {key} -o StrictHostKeyChecking=accept-new -o ConnectTimeout=5 root@{ip} \
      "test -f /mnt/data/.initialized" 2>/dev/null; then
    echo "  Cloud-init complete after ${{i}}x10s"
    break
  fi
  if [ "$i" -eq 60 ]; then
    echo "ERROR: Cloud-init did not complete after 10 minutes."
    echo "Check logs: cargo xtask infra ssh, then: tail -f /var/log/cloud-init-output.log"
    exit 1
  fi
  printf "  Waiting... ($i/60)\r"
  sleep 10
done

echo ""
echo "=== Credentials ==="
ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} "/opt/bdp/scripts/show-secrets.sh"
"#,
        ip = ip,
        key = key
    );
    #[cfg(not(target_os = "windows"))]
    return run_bash(&script, "Post-deploy");
    #[cfg(target_os = "windows")]
    return run_powershell(
        &format!("wsl bash -c '{}'", script.replace('\'', "'\\''")),
        "Post-deploy",
    );
}

fn show_secrets() -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    let script = format!(
        r#"ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} "/opt/bdp/scripts/show-secrets.sh""#,
        ip = ip,
        key = key
    );
    #[cfg(not(target_os = "windows"))]
    return run_bash(&script, "Show secrets");
    #[cfg(target_os = "windows")]
    return run_powershell(
        &format!(
            "ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} \"/opt/bdp/scripts/show-secrets.sh\"",
            ip = ip,
            key = key
        ),
        "Show secrets",
    );
}

fn backup_now() -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    let script = format!(
        r#"
echo "Triggering restic backup on {ip}..."
ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} \
  "MOUNT_POINT=/mnt/data /opt/bdp/scripts/backup-restic.sh"
"#,
        ip = ip,
        key = key
    );
    #[cfg(not(target_os = "windows"))]
    return run_bash(&script, "Backup now");
    #[cfg(target_os = "windows")]
    return run_powershell(
        &format!(
            "ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} \
             \"MOUNT_POINT=/mnt/data /opt/bdp/scripts/backup-restic.sh\"",
            ip = ip,
            key = key
        ),
        "Backup now",
    );
}

fn backup_list() -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    let script = format!(
        r#"
echo "Restic snapshots on {ip}:"
ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} \
  "source /mnt/data/.secrets/env && restic snapshots --repo \$RESTIC_REPOSITORY"
"#,
        ip = ip,
        key = key
    );
    #[cfg(not(target_os = "windows"))]
    return run_bash(&script, "Backup list");
    #[cfg(target_os = "windows")]
    return run_powershell(
        &format!(
            "ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} \
             \"source /mnt/data/.secrets/env && restic snapshots --repo \\$RESTIC_REPOSITORY\"",
            ip = ip,
            key = key
        ),
        "Backup list",
    );
}

fn backup_restore() -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    println!("WARNING: This will restore files from a restic snapshot.");
    println!("Run the following to restore interactively:");
    println!();
    println!("  ssh -i {} root@{} \\", key, ip);
    println!("    'source /mnt/data/.secrets/env && restic restore latest --target /mnt/data'");
    println!();
    println!("Or to restore to a temporary location first:");
    println!("  ssh -i {} root@{} \\", key, ip);
    println!("    'source /mnt/data/.secrets/env && restic restore latest --target /tmp/restore'");
    Ok(())
}

fn logs(service: &str) -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    let script = format!(
        r#"ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} "docker logs -f --tail=100 {service}""#,
        ip = ip,
        key = key,
        service = service
    );
    #[cfg(not(target_os = "windows"))]
    return run_bash(&script, &format!("Logs for {service}"));
    #[cfg(target_os = "windows")]
    return run_powershell(
        &format!(
            "ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} \"docker logs -f --tail=100 {service}\"",
            ip = ip,
            key = key,
            service = service
        ),
        &format!("Logs for {service}"),
    );
}

fn update_services() -> Result<()> {
    let ip = get_server_ip()?;
    let key = ssh_key_path();
    let script = format!(
        r#"
echo "Pulling latest images and restarting services on {ip}..."
ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} "
  docker pull ghcr.io/datadir-lab/bdp-server:latest
  docker pull ghcr.io/datadir-lab/bdp-web:latest
  docker restart bdp-server bdp-web
  docker ps --format 'table {{{{.Names}}}}\t{{{{.Status}}}}'
"
echo "Services updated."
"#,
        ip = ip,
        key = key
    );
    #[cfg(not(target_os = "windows"))]
    return run_bash(&script, "Update services");
    #[cfg(target_os = "windows")]
    return run_powershell(
        &format!(
            "ssh -i {key} -o StrictHostKeyChecking=accept-new root@{ip} \
             \"docker pull ghcr.io/datadir-lab/bdp-server:latest; \
               docker pull ghcr.io/datadir-lab/bdp-web:latest; \
               docker restart bdp-server bdp-web; \
               docker ps --format 'table {{{{.Names}}}}\t{{{{.Status}}}}'\"",
            ip = ip,
            key = key
        ),
        "Update services",
    );
}
