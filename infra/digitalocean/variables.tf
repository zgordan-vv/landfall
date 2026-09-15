variable "name" {
  description = "Short, unique deployment name; used for every cloud resource."
  type        = string
  default     = "landfall-production"

  validation {
    condition     = can(regex("^[a-z0-9][a-z0-9-]{1,61}[a-z0-9]$", var.name))
    error_message = "name must be a 3–63 character lowercase DigitalOcean-safe slug."
  }
}

variable "region" {
  description = "DigitalOcean region for the Droplet, VPC, and database."
  type        = string
  default     = "fra1"
}

variable "droplet_size" {
  description = "Droplet size slug. Use a production-appropriate plan, not a demo-sized host."
  type        = string
  default     = "s-2vcpu-4gb"
}

variable "database_size" {
  description = "Managed PostgreSQL size slug."
  type        = string
  default     = "db-s-1vcpu-1gb"
}

variable "database_version" {
  description = "Supported PostgreSQL major version selected in the DigitalOcean account."
  type        = string
  default     = "16"
}

variable "ssh_key_fingerprints" {
  description = "Existing DigitalOcean SSH-key fingerprints allowed to administer the Droplet."
  type        = set(string)

  validation {
    condition     = length(var.ssh_key_fingerprints) > 0
    error_message = "At least one existing SSH key fingerprint is required."
  }
}

variable "admin_ipv4_cidrs" {
  description = "IPv4 CIDRs allowed to reach SSH. Keep this to the operator's fixed address or VPN range."
  type        = set(string)

  validation {
    condition     = length(var.admin_ipv4_cidrs) > 0 && alltrue([for cidr in var.admin_ipv4_cidrs : can(cidrhost(cidr, 0))])
    error_message = "Provide at least one valid IPv4 CIDR, for example 203.0.113.10/32."
  }
}

variable "domain_name" {
  description = "Already-registered hostname to point at the public Droplet IP, for example app.example.com."
  type        = string

  validation {
    condition     = can(regex("^[a-z0-9][a-z0-9.-]*[a-z0-9]$", var.domain_name))
    error_message = "domain_name must be a lowercase hostname without a protocol or path."
  }
}

variable "dns_zone_name" {
  description = "Apex domain already delegated to DigitalOcean DNS, for example example.com."
  type        = string

  validation {
    condition     = can(regex("^[a-z0-9][a-z0-9.-]*[a-z0-9]$", var.dns_zone_name)) && (var.domain_name == var.dns_zone_name || endswith(var.domain_name, ".${var.dns_zone_name}"))
    error_message = "dns_zone_name must be the delegated apex zone and domain_name must belong to it."
  }
}
