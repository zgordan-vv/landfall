output "application_ipv4" {
  description = "Public IP of the TLS-terminating Landfall Droplet."
  value       = digitalocean_droplet.application.ipv4_address
}

output "database_private_host" {
  description = "VPC-only PostgreSQL hostname. Do not use the public endpoint for the application."
  value       = digitalocean_database_cluster.postgres.private_host
}

output "database_private_uri" {
  description = "Sensitive VPC-only TLS connection URI for the Docker secret file."
  value       = digitalocean_database_cluster.postgres.private_uri
  sensitive   = true
}
