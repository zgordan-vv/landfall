//! Encryption boundary for customer-controlled RPC endpoints.

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use getrandom::fill;
use sqlx::{PgPool, Row};

/// AES-256-GCM key loaded only from deployment configuration.
#[derive(Clone)]
pub struct RouteSecretCipher {
    cipher: Aes256Gcm,
}

impl RouteSecretCipher {
    /// Loads a 32-byte hex key from a file (preferred) or environment value.
    pub fn from_env() -> Result<Option<Self>, SecretConfigError> {
        let value = match std::env::var("LANDFALL_ROUTE_SECRETS_KEY_FILE") {
            Ok(path) => {
                std::fs::read_to_string(path).map_err(|_| SecretConfigError::UnreadableKey)?
            }
            Err(_) => match std::env::var("LANDFALL_ROUTE_SECRETS_KEY") {
                Ok(value) => value,
                Err(_) => return Ok(None),
            },
        };
        let bytes = decode_hex_key(value.trim()).ok_or(SecretConfigError::InvalidKey)?;
        Ok(Some(Self {
            cipher: Aes256Gcm::new_from_slice(&bytes).map_err(|_| SecretConfigError::InvalidKey)?,
        }))
    }

    /// Encrypts one endpoint using a unique 96-bit nonce.
    pub fn encrypt(&self, endpoint: &str) -> Result<(Vec<u8>, Vec<u8>), SecretCryptoError> {
        let mut nonce = [0_u8; 12];
        fill(&mut nonce).map_err(|_| SecretCryptoError)?;
        let ciphertext = self
            .cipher
            .encrypt(Nonce::from_slice(&nonce), endpoint.as_bytes())
            .map_err(|_| SecretCryptoError)?;
        Ok((ciphertext, nonce.to_vec()))
    }

    /// Decrypts an endpoint only inside the worker or connection-check path.
    pub fn decrypt(&self, ciphertext: &[u8], nonce: &[u8]) -> Result<String, SecretCryptoError> {
        if nonce.len() != 12 {
            return Err(SecretCryptoError);
        }
        let bytes = self
            .cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|_| SecretCryptoError)?;
        String::from_utf8(bytes).map_err(|_| SecretCryptoError)
    }
}

/// Moves old plaintext endpoint rows into encrypted columns during rollout.
pub async fn encrypt_legacy_routes(
    pool: &PgPool,
    cipher: &RouteSecretCipher,
) -> Result<u64, sqlx::Error> {
    let rows = sqlx::query("SELECT route_id, endpoint FROM control.routes WHERE endpoint IS NOT NULL AND endpoint_ciphertext IS NULL FOR UPDATE")
        .fetch_all(pool).await?;
    let mut converted = 0_u64;
    for row in rows {
        let route_id: uuid::Uuid = row.get("route_id");
        let endpoint: String = row.get("endpoint");
        let Ok((ciphertext, nonce)) = cipher.encrypt(&endpoint) else {
            continue;
        };
        let result = sqlx::query("UPDATE control.routes SET endpoint = NULL, endpoint_ciphertext = $1, endpoint_nonce = $2, encryption_key_id = 'v1' WHERE route_id = $3 AND endpoint_ciphertext IS NULL")
            .bind(ciphertext).bind(nonce).bind(route_id).execute(pool).await?;
        converted += result.rows_affected();
    }
    Ok(converted)
}

fn decode_hex_key(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}

/// Missing or invalid deployment secret configuration.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SecretConfigError {
    UnreadableKey,
    InvalidKey,
}
impl std::fmt::Display for SecretConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LANDFALL_ROUTE_SECRETS_KEY must be a 32-byte hex key")
    }
}
impl std::error::Error for SecretConfigError {}
#[derive(Debug, Clone, Copy)]
pub struct SecretCryptoError;

#[cfg(test)]
mod tests {
    use super::{RouteSecretCipher, decode_hex_key};
    use aes_gcm::aead::KeyInit;
    #[test]
    fn encrypts_with_unique_nonce_and_round_trips() {
        assert!(decode_hex_key("ab".repeat(32).as_str()).is_some());
        let value = RouteSecretCipher {
            cipher: aes_gcm::Aes256Gcm::new_from_slice(&[7_u8; 32]).expect("key"),
        };
        let (encrypted, nonce) = value
            .encrypt("https://private.rpc.example/key")
            .expect("encrypt");
        assert!(!String::from_utf8_lossy(&encrypted).contains("private"));
        assert_eq!(
            value.decrypt(&encrypted, &nonce).expect("decrypt"),
            "https://private.rpc.example/key"
        );
    }
}
