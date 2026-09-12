# Secure headers and CORS

The server adds `nosniff`, `DENY` frame protection, `no-referrer`, restrictive
CSP, and a permissions policy to every response. Cross-origin requests are
rejected unless the `Origin` host exactly matches the request `Host`; wildcard
origins and credentialed cross-site access are not enabled by default.
