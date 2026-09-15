# DigitalOcean production infrastructure

This is deployable infrastructure, not a local demo. `terraform apply` creates:

- one public Ubuntu 24.04 Droplet for Landfall and nginx;
- one VPC-isolated DigitalOcean Managed PostgreSQL cluster;
- a database firewall that accepts only the VPC range;
- a Droplet firewall exposing only HTTPS, HTTP for certificate issuance, and
  SSH from the administrator CIDRs; and
- an `A` DNS record in a domain zone already delegated to DigitalOcean DNS.

The application never receives the database's public connection URI. The
deployment uses the Managed PostgreSQL VPC-only TLS URI as a Docker secret.

## One-time setup

1. In DigitalOcean, add an SSH public key and create a personal access token
   with read/write scope. Keep the token in your password manager; do not put
   it in Git or send it in chat.
2. Register a domain if needed, then delegate its DNS zone to DigitalOcean.
   The zone must exist in the account before Terraform can create its record.
3. Copy `terraform.tfvars.example` to `terraform.tfvars`, replace every
   example value, and restrict `admin_ipv4_cidrs` to your public IP or VPN.
4. From this directory, authenticate only in the current terminal and review
   the plan before any paid resources are created:

   ```bash
   export DIGITALOCEAN_ACCESS_TOKEN='paste-from-password-manager'
   terraform init
   terraform plan
   terraform apply
   ```

`apply` is deliberately not run by Landfall or CI: it creates billable cloud
resources in your account.

## Deploy after Terraform finishes

Use the `application_ipv4` Terraform output to SSH to the Droplet. Copy the
repository or a verified release archive to `/srv/landfall/repository`, then
on that host create the deployment-only secrets directory:

```bash
install -d -m 700 /srv/landfall/secrets
install -m 600 /dev/null /srv/landfall/secrets/database-url
install -m 600 /dev/null /srv/landfall/secrets/bootstrap-token
```

Put the sensitive `database_private_uri` Terraform output in `database-url`.
It is a VPC-only URI and should retain TLS verification parameters. Generate a
random bootstrap token locally and write it to `bootstrap-token`. Copy
`../../deploy/digitalocean.env.example` to `/srv/landfall/production.env`, set
the immutable release digest, configure nginx for `domain_name`, obtain the
TLS certificate, then run:

```bash
cd /srv/landfall/repository
LANDFALL_DEPLOY_TARGET=managed-postgres \
LANDFALL_DEPLOYMENT_NAME=landfall \
LANDFALL_DEPLOY_ENV_FILE=/srv/landfall/production.env \
LANDFALL_DATABASE_URL_FILE=/srv/landfall/secrets/database-url \
LANDFALL_BOOTSTRAP_TOKEN_SECRET_FILE=/srv/landfall/secrets/bootstrap-token \
  ./scripts/deploy-release.sh
```

The command starts the application, observer worker, and Prometheus but never
starts the repository's `postgres-production` service. Verify
`https://YOUR_DOMAIN/health/ready` and complete an authenticated ingest before
routing real customer traffic.
