-- RPC endpoint secrets are encrypted by the application before persistence.
-- `endpoint` remains temporarily nullable only so a startup migration can
-- safely convert an existing self-hosted installation without losing routes.
ALTER TABLE control.routes
    ALTER COLUMN endpoint DROP NOT NULL;

ALTER TABLE control.routes
    ADD COLUMN IF NOT EXISTS endpoint_ciphertext bytea,
    ADD COLUMN IF NOT EXISTS endpoint_nonce bytea,
    ADD COLUMN IF NOT EXISTS encryption_key_id text;

ALTER TABLE control.routes
    ADD CONSTRAINT routes_encrypted_endpoint_shape
    CHECK (
        (endpoint_ciphertext IS NULL AND endpoint_nonce IS NULL AND encryption_key_id IS NULL)
        OR (
            endpoint_ciphertext IS NOT NULL
            AND octet_length(endpoint_ciphertext) BETWEEN 17 AND 4096
            AND endpoint_nonce IS NOT NULL
            AND octet_length(endpoint_nonce) = 12
            AND encryption_key_id = 'v1'
        )
    );

ALTER TABLE control.api_tokens
    ADD COLUMN IF NOT EXISTS last_used_at timestamptz,
    ADD COLUMN IF NOT EXISTS last_used_ip_hash bytea;

ALTER TABLE control.api_tokens
    ADD CONSTRAINT api_tokens_last_used_ip_hash_size
    CHECK (last_used_ip_hash IS NULL OR octet_length(last_used_ip_hash) = 32);
