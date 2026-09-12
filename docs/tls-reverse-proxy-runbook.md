# TLS reverse-proxy runbook

`config/nginx/landfall.conf.example` is an nginx server-block example for a
public Landfall deployment. It redirects HTTP to HTTPS, enables TLS 1.2/1.3,
adds HSTS, limits request size/rate, and forwards the original scheme and
client address to the private server listener.

Before enabling it:

1. Replace the example hostname and certificate paths.
2. Ensure the certificate key is readable only by nginx.
3. Strip any client-provided `Forwarded`/`X-Forwarded-*` headers at the edge and
   trust the values set by this proxy only.
4. Keep PostgreSQL off the public network and do not log `Authorization` or
   raw RPC URLs.
5. Run `nginx -t` and perform a HTTPS smoke test for `/health/live` and
   `/health/ready` before switching DNS.

The file is intentionally a configuration fragment (`limit_req_zone` belongs
inside nginx's `http` context); it is not a drop-in production config without
certificate, hostname, and upstream review.
