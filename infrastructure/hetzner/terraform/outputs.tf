output "server_ipv4" {
  description = "Server public IPv4"
  value       = hcloud_primary_ip.main_ipv4.ip_address
}

output "server_id" {
  description = "Hetzner server ID"
  value       = hcloud_server.main.id
}

output "volume_id" {
  description = "Data volume ID"
  value       = hcloud_volume.data.id
}

output "storage_box_host" {
  description = "Restic backup host"
  value       = hcloud_storage_box.backup.server
}

output "storage_box_user" {
  description = "Restic backup username"
  value       = hcloud_storage_box.backup.username
}

output "dokploy_url" {
  description = "Dokploy management UI"
  value       = "https://dokploy.${var.domain}"
}

output "app_url" {
  description = "BDP application URL"
  value       = "https://${var.domain}"
}

output "ssh_command" {
  description = "SSH command to connect to server"
  value       = "ssh root@${hcloud_primary_ip.main_ipv4.ip_address}"
}
