provider "cloudflare" {
  api_token = var.cloudflare_api_token
}

# Look up the zone ID from the domain name.
# NOTE: cloudflare provider v4 uses `cloudflare_zone` (singular), NOT `cloudflare_zones`.
data "cloudflare_zone" "domain" {
  count = var.create_dns_record && var.cloudflare_api_token != "" ? 1 : 0
  name  = var.domain
}

# A record: bdp.dev → server IP
resource "cloudflare_record" "apex" {
  count   = var.create_dns_record && var.cloudflare_api_token != "" ? 1 : 0
  zone_id = data.cloudflare_zone.domain[0].id
  name    = "@"
  type    = "A"
  content = hcloud_primary_ip.main_ipv4.ip_address
  ttl     = 300
  proxied = false # Direct — Traefik handles TLS
}

# A record: *.bdp.dev → server IP (for dokploy.bdp.dev etc.)
resource "cloudflare_record" "wildcard" {
  count   = var.create_dns_record && var.cloudflare_api_token != "" ? 1 : 0
  zone_id = data.cloudflare_zone.domain[0].id
  name    = "*"
  type    = "A"
  content = hcloud_primary_ip.main_ipv4.ip_address
  ttl     = 300
  proxied = false
}
