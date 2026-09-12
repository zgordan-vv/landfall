# Secret fixture matrix

`scan_secret_matrix` accepts representative serialized payloads from SDK, API,
raw projection, logs, UI, and report surfaces. Each payload is checked with the
same recursive scanner for credential-bearing keys/values. A clean fixture must
produce zero findings; when a secret appears, the result identifies only the
surface name, never the secret itself.
