use std::path::Path;
use std::sync::Arc;

use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2, Params,
};
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::crypto::EncryptionKey;
use crate::errors::{RcAuthError, Result};
use crate::secret::SecretProvider;

const SALT_LEN: usize = 32;

/// Metadata for key derivation and storage format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMeta {
    pub version: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Base64-encoded salt for Argon2id (if using passphrase)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passphrase_salt: Option<String>,
}

impl Default for KeyMeta {
    fn default() -> Self {
        Self {
            version: 1,
            created_at: chrono::Utc::now(),
            passphrase_salt: None,
        }
    }
}

/// Manages encryption keys with OS keyring and passphrase fallback
pub struct KeyManager {
    meta: KeyMeta,
    key: EncryptionKey,
    secret_provider: Arc<dyn SecretProvider>,
}

impl KeyManager {
    /// Create a new key manager with OS keyring
    /// 
    /// Tries to load key from OS keyring first. If not found or keyring unavailable,
    /// falls back to passphrase-derived key.
    #[cfg(feature = "keyring-support")]
    pub async fn new(
        storage_dir: &Path,
        secret_provider: Arc<dyn SecretProvider>,
    ) -> Result<Self> {
        let meta_path = storage_dir.join("meta.json");

        // Try to load existing metadata
        let mut meta = if meta_path.exists() {
            let content = fs::read_to_string(&meta_path).await?;
            serde_json::from_str(&content).map_err(|e| {
                RcAuthError::InvalidResponse(format!("Invalid meta.json: {}", e))
            })?
        } else {
            KeyMeta::default()
        };

        // Try OS keyring first
        let key = match Self::load_from_keyring() {
            Ok(key) => {
                tracing::debug!("Loaded encryption key from OS keyring");
                key
            }
            Err(e) => {
                tracing::debug!("Keyring unavailable ({}), using passphrase fallback", e);
                
                // Try passphrase fallback
                let key = Self::derive_from_passphrase(&mut meta, &secret_provider).await?;
                
                // Try to save to keyring for next time
                if let Err(e) = Self::save_to_keyring(&key) {
                    tracing::warn!("Failed to save key to keyring: {}", e);
                }
                
                key
            }
        };

        // Save metadata
        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| RcAuthError::InvalidResponse(format!("Failed to serialize meta: {}", e)))?;
        fs::write(&meta_path, meta_json).await?;

        Ok(Self {
            meta,
            key,
            secret_provider,
        })
    }

    /// Create a new key manager without keyring support
    #[cfg(not(feature = "keyring-support"))]
    pub async fn new(
        storage_dir: &Path,
        secret_provider: Arc<dyn SecretProvider>,
    ) -> Result<Self> {
        let meta_path = storage_dir.join("meta.json");

        // Try to load existing metadata
        let mut meta = if meta_path.exists() {
            let content = fs::read_to_string(&meta_path).await?;
            serde_json::from_str(&content).map_err(|e| {
                RcAuthError::InvalidResponse(format!("Invalid meta.json: {}", e))
            })?
        } else {
            KeyMeta::default()
        };

        let key = Self::derive_from_passphrase(&mut meta, &secret_provider).await?;

        // Save metadata
        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| RcAuthError::InvalidResponse(format!("Failed to serialize meta: {}", e)))?;
        fs::write(&meta_path, meta_json).await?;

        Ok(Self {
            meta,
            key,
            secret_provider,
        })
    }

    /// Get the encryption key
    pub fn key(&self) -> &EncryptionKey {
        &self.key
    }

    /// Load key from OS keyring
    #[cfg(feature = "keyring-support")]
    fn load_from_keyring() -> Result<EncryptionKey> {
        let entry = keyring::Entry::new("rauncher-mc", "rc-auth:v1")
            .map_err(|e| RcAuthError::Keyring(format!("Failed to access keyring: {}", e)))?;

        let key_b64 = entry
            .get_password()
            .map_err(|e| RcAuthError::Keyring(format!("Failed to read from keyring: {}", e)))?;

        let key_bytes = base64::engine::general_purpose::STANDARD
            .decode(key_b64)
            .map_err(|_| RcAuthError::CorruptedStore)?;

        if key_bytes.len() != 32 {
            return Err(RcAuthError::CorruptedStore);
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);

        Ok(EncryptionKey::from_bytes(key))
    }

    /// Save key to OS keyring
    #[cfg(feature = "keyring-support")]
    fn save_to_keyring(key: &EncryptionKey) -> Result<()> {
        let entry = keyring::Entry::new("rauncher-mc", "rc-auth:v1")
            .map_err(|e| RcAuthError::Keyring(format!("Failed to access keyring: {}", e)))?;

        let key_b64 = base64::engine::general_purpose::STANDARD.encode(key.as_bytes());

        entry
            .set_password(&key_b64)
            .map_err(|e| RcAuthError::Keyring(format!("Failed to write to keyring: {}", e)))?;

        Ok(())
    }

    /// Derive key from passphrase using Argon2id
    async fn derive_from_passphrase(
        meta: &mut KeyMeta,
        secret_provider: &Arc<dyn SecretProvider>,
    ) -> Result<EncryptionKey> {
        // Get or generate salt
        let salt = if let Some(ref salt_b64) = meta.passphrase_salt {
            base64::engine::general_purpose::STANDARD
                .decode(salt_b64)
                .map_err(|_| RcAuthError::CorruptedStore)?
        } else {
            // Generate new salt
            let mut salt = vec![0u8; SALT_LEN];
            rand::rngs::OsRng.fill_bytes(&mut salt);
            meta.passphrase_salt = Some(base64::engine::general_purpose::STANDARD.encode(&salt));
            salt
        };

        // Get passphrase from provider
        let passphrase = secret_provider
            .get_passphrase("Enter passphrase for token storage")
            .await
            .ok_or(RcAuthError::UserCancelled)?;

        // Derive key using Argon2id
        // Parameters: m=64MB, t=3, p=1
        let params = Params::new(65536, 3, 1, Some(32))
            .map_err(|e| RcAuthError::Crypto(format!("Invalid Argon2 params: {}", e)))?;
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            params,
        );

        let salt_string = SaltString::encode_b64(&salt)
            .map_err(|e| RcAuthError::Crypto(format!("Invalid salt: {}", e)))?;

        let hash = argon2
            .hash_password(passphrase.as_bytes(), &salt_string)
            .map_err(|e| RcAuthError::Crypto(format!("Key derivation failed: {}", e)))?;

        let key_bytes = hash.hash.ok_or_else(|| {
            RcAuthError::Crypto("Argon2 hash returned no output".to_string())
        })?;

        if key_bytes.len() != 32 {
            return Err(RcAuthError::Crypto(format!(
                "Expected 32 bytes, got {}",
                key_bytes.len()
            )));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(key_bytes.as_bytes());

        Ok(EncryptionKey::from_bytes(key))
    }

    /// Rotate the encryption key (re-encrypt all data)
    /// 
    /// This should be called by the FileTokenStore to re-encrypt all sessions.
    pub async fn rotate(&mut self, storage_dir: &Path) -> Result<EncryptionKey> {
        // Generate new key
        let new_key = EncryptionKey::generate();

        // Update metadata
        self.meta.created_at = chrono::Utc::now();

        // Try to save to keyring
        #[cfg(feature = "keyring-support")]
        {
            if let Err(e) = Self::save_to_keyring(&new_key) {
                tracing::warn!("Failed to save new key to keyring: {}", e);
            }
        }

        // Save metadata
        let meta_path = storage_dir.join("meta.json");
        let meta_json = serde_json::to_string_pretty(&self.meta)
            .map_err(|e| RcAuthError::InvalidResponse(format!("Failed to serialize meta: {}", e)))?;
        fs::write(&meta_path, meta_json).await?;

        Ok(new_key)
    }
}

impl std::fmt::Debug for KeyManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyManager")
            .field("meta", &self.meta)
            .field("key", &"[REDACTED]")
            .finish()
    }
}
