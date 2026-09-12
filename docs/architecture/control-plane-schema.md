# Control-plane schema

Migration `0002_create_control_entities.sql` adds the project hierarchy:

```text
project → environment → route
project → api_token
```

Foreign keys prevent cross-project/environment references. API credentials are
represented by a bounded prefix and salted/hash material (`bytea`); plaintext
tokens never enter PostgreSQL. Expiry and revocation timestamps support
rotation without deleting audit history.
