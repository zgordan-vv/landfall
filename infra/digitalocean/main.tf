locals {
  tags = ["landfall", "production", var.name]
}

resource "digitalocean_project" "landfall" {
  name        = var.name
  description = "Landfall production transaction-observability service"
  purpose     = "Service or API"
  environment = "Production"
}

resource "digitalocean_vpc" "landfall" {
  name     = "${var.name}-vpc"
  region   = var.region
  ip_range = "10.42.0.0/24"
}

resource "digitalocean_droplet" "application" {
  name       = "${var.name}-app-1"
  region     = var.region
  size       = var.droplet_size
  image      = "ubuntu-24-04-x64"
  vpc_uuid   = digitalocean_vpc.landfall.id
  ssh_keys   = var.ssh_key_fingerprints
  monitoring = true
  tags       = local.tags
  user_data  = file("${path.module}/cloud-init.yaml")

  lifecycle {
    prevent_destroy = true
  }
}

resource "digitalocean_database_cluster" "postgres" {
  name                 = "${var.name}-postgres"
  engine               = "pg"
  version              = var.database_version
  size                 = var.database_size
  region               = var.region
  node_count           = 1
  private_network_uuid = digitalocean_vpc.landfall.id
  tags                 = local.tags

  lifecycle {
    prevent_destroy = true
  }
}

resource "digitalocean_database_db" "landfall" {
  cluster_id = digitalocean_database_cluster.postgres.id
  name       = "landfall"
}

resource "digitalocean_database_firewall" "postgres" {
  cluster_id = digitalocean_database_cluster.postgres.id

  rule {
    type  = "ip_addr"
    value = digitalocean_vpc.landfall.ip_range
  }
}

resource "digitalocean_firewall" "application" {
  name        = "${var.name}-app"
  droplet_ids = [digitalocean_droplet.application.id]
  tags        = local.tags

  inbound_rule {
    protocol         = "tcp"
    port_range       = "22"
    source_addresses = tolist(var.admin_ipv4_cidrs)
  }

  inbound_rule {
    protocol         = "tcp"
    port_range       = "80"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }

  inbound_rule {
    protocol         = "tcp"
    port_range       = "443"
    source_addresses = ["0.0.0.0/0", "::/0"]
  }

  outbound_rule {
    protocol              = "tcp"
    port_range            = "1-65535"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }

  outbound_rule {
    protocol              = "udp"
    port_range            = "1-65535"
    destination_addresses = ["0.0.0.0/0", "::/0"]
  }
}

resource "digitalocean_record" "application" {
  count  = var.domain_name == "" ? 0 : 1
  domain = var.dns_zone_name
  type   = "A"
  name   = var.domain_name == var.dns_zone_name ? "@" : trimsuffix(var.domain_name, ".${var.dns_zone_name}")
  value  = digitalocean_droplet.application.ipv4_address
  ttl    = 300
}

resource "digitalocean_project_resources" "landfall" {
  project   = digitalocean_project.landfall.id
  resources = [digitalocean_droplet.application.urn, digitalocean_database_cluster.postgres.urn]
}
