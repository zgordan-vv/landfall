# TLS and reverse-proxy deployment review

Non-loopback deployments must terminate TLS at a managed reverse proxy (or use
a trusted private network). Cleartext bearer tokens over an untrusted network
are unsupported.

Required edge settings:

- redirect HTTP to HTTPS and enable HSTS after certificate rollout;
- strip client-supplied `Forwarded`/`X-Forwarded-*` headers, then set them from
  the trusted proxy address;
- enforce request/body and connection-rate limits before forwarding;
- do not log `Authorization`, cookies, query tokens, or raw RPC URLs;
- proxy only `/api` and health paths; keep PostgreSQL private;
- validate certificates and use HTTP/2 where supported.

The upstream server should bind to loopback/private networking, while the proxy
owns the public certificate, origin policy, and access logs.
