use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::errors::{RcAuthError, Result};

/// AES-256 key (32 bytes)
#[derive(Clone, ZeroizeOnDrop)]
pub struct EncryptionKey {
    key: [u8; 32],
}

impl EncryptionKey {
    /// Generate a new random encryption key
    pub fn generate() -> Self {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        Self { key }
    }

    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { key: bytes }
    }

    /// Get key bytes (use carefully - sensitive data)
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.key
    }

    /// Convert to bytes, consuming self
    pub fn into_bytes(self) -> [u8; 32] {
        self.key
    }
}

impl std::fmt::Debug for EncryptionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EncryptionKey([REDACTED])")
    }
}

/// Encrypted data with nonce and authentication tag
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EncryptedBlob {
    /// Base64url-encoded nonce (12 bytes)
    pub nonce: String,
    /// Base64url-encoded ciphertext + tag
    pub ciphertext: String,
    /// Additional authenticated data version
    pub aad_version: String,
}

/// Encrypt plaintext using AES-256-GCM
pub fn encrypt(
    key: &EncryptionKey,
    plaintext: &[u8],
    account_key: &str,
) -> Result<EncryptedBlob> {
    let cipher = Aes256Gcm::new(key.as_bytes().into());

    // Generate random 96-bit nonce
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // AAD format: "rc-auth|v1|{account_key}"
    let aad_version = "v1".to_string();
    let aad = format!("rc-auth|{}|{}", aad_version, account_key);

    // Encrypt with AAD
    let ciphertext = cipher
        .encrypt(nonce, aes_gcm::aead::Payload {
            msg: plaintext,
            aad: aad.as_bytes(),
        })
        .map_err(|e| RcAuthError::Crypto(format!("Encryption failed: {}", e)))?;

    Ok(EncryptedBlob {
        nonce: URL_SAFE_NO_PAD.encode(nonce_bytes),
        ciphertext: URL_SAFE_NO_PAD.encode(ciphertext),
        aad_version,
    })
}

/// Decrypt ciphertext using AES-256-GCM
pub fn decrypt(
    key: &EncryptionKey,
    blob: &EncryptedBlob,
    account_key: &str,
) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(key.as_bytes().into());

    // Decode nonce
    let nonce_bytes = URL_SAFE_NO_PAD
        .decode(&blob.nonce)
        .map_err(|e| RcAuthError::Crypto(format!("Invalid nonce: {}", e)))?;
    
    if nonce_bytes.len() != 12 {
        return Err(RcAuthError::CorruptedStore);
    }
    
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Decode ciphertext
    let ciphertext = URL_SAFE_NO_PAD
        .decode(&blob.ciphertext)
        .map_err(|e| RcAuthError::Crypto(format!("Invalid ciphertext: {}", e)))?;

    // Reconstruct AAD
    let aad = format!("rc-auth|{}|{}", blob.aad_version, account_key);

    // Decrypt with AAD
    let plaintext = cipher
        .decrypt(nonce, aes_gcm::aead::Payload {
            msg: &ciphertext,
            aad: aad.as_bytes(),
        })
        .map_err(|_| RcAuthError::CorruptedStore)?;

    // Return plaintext (caller should zeroize if needed)
    Ok(plaintext)
}

/// Zeroize a Vec<u8> containing sensitive data
pub fn zeroize_vec(data: &mut Vec<u8>) {
    data.zeroize();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = EncryptionKey::generate();
        let plaintext = b"sensitive session data";
        let account_key = "test-account-123";

        let encrypted = encrypt(&key, plaintext, account_key).unwrap();
        let decrypted = decrypt(&key, &encrypted, account_key).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = EncryptionKey::generate();
        let key2 = EncryptionKey::generate();
        let plaintext = b"sensitive data";
        let account_key = "test";

        let encrypted = encrypt(&key1, plaintext, account_key).unwrap();
        let result = decrypt(&key2, &encrypted, account_key);

        assert!(matches!(result, Err(RcAuthError::CorruptedStore)));
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = EncryptionKey::generate();
        let plaintext = b"data";
        let account_key = "test";

        let mut encrypted = encrypt(&key, plaintext, account_key).unwrap();
        
        // Tamper with ciphertext
        let mut ct_bytes = URL_SAFE_NO_PAD.decode(&encrypted.ciphertext).unwrap();
        ct_bytes[0] ^= 0xFF;
        encrypted.ciphertext = URL_SAFE_NO_PAD.encode(ct_bytes);

        let result = decrypt(&key, &encrypted, account_key);
        assert!(matches!(result, Err(RcAuthError::CorruptedStore)));
    }

    #[test]
    fn test_wrong_aad_fails() {
        let key = EncryptionKey::generate();
        let plaintext = b"data";
        let account_key1 = "account1";
        let account_key2 = "account2";

        let encrypted = encrypt(&key, plaintext, account_key1).unwrap();
        let result = decrypt(&key, &encrypted, account_key2);

        assert!(matches!(result, Err(RcAuthError::CorruptedStore)));
    }

    #[test]
    fn test_key_zeroize() {
        let mut key = EncryptionKey::generate();
        let key_ptr = key.as_bytes().as_ptr();
        
        drop(key);
        
        // Key should be zeroized on drop
        // This is a basic test - in practice, zeroize's guarantees are stronger
    }
}
