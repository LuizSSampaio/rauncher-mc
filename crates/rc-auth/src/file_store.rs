use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use fs2::FileExt;
use tokio::fs;
use tokio::sync::RwLock;

use crate::crypto::{self, EncryptedBlob};
use crate::errors::{RcAuthError, Result};
use crate::key_manager::KeyManager;
use crate::secret::SecretProvider;
use crate::session::Session;
use crate::store::TokenStore;

/// File-based encrypted token store
///
/// Stores encrypted authentication sessions in per-account files.
/// Uses OS keyring for key management with passphrase fallback.
///
/// # Directory Structure
/// ```text
/// ~/.config/rauncher/rc-auth/
/// ├── meta.json              # Storage metadata
/// ├── lock                   # Advisory lock file
/// └── accounts/
///     ├── uuid1.json         # Encrypted session for account 1
///     └── uuid2.json         # Encrypted session for account 2
/// ```
#[derive(Debug)]
pub struct FileTokenStore {
    storage_dir: PathBuf,
    accounts_dir: PathBuf,
    lock_file: PathBuf,
    key_manager: Arc<RwLock<KeyManager>>,
    /// In-memory cache for recently accessed sessions
    cache: Arc<RwLock<HashMap<String, Session>>>,
}

impl FileTokenStore {
    /// Create a new file-based token store
    ///
    /// # Arguments
    /// * `storage_dir` - Base directory for storage (e.g., ~/.config/rauncher/rc-auth)
    /// * `secret_provider` - Provider for passphrase fallback
    pub async fn new(
        storage_dir: impl AsRef<Path>,
        secret_provider: Arc<dyn SecretProvider>,
    ) -> Result<Self> {
        let storage_dir = storage_dir.as_ref().to_path_buf();
        let accounts_dir = storage_dir.join("accounts");
        let lock_file = storage_dir.join("lock");

        // Create directories
        fs::create_dir_all(&storage_dir).await?;
        fs::create_dir_all(&accounts_dir).await?;

        // Set secure permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(&storage_dir, perms.clone())?;
            std::fs::set_permissions(&accounts_dir, perms)?;
        }

        // Initialize key manager
        let key_manager = KeyManager::new(&storage_dir, secret_provider).await?;

        Ok(Self {
            storage_dir,
            accounts_dir,
            lock_file,
            key_manager: Arc::new(RwLock::new(key_manager)),
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get default storage directory for the current platform
    pub fn default_storage_dir() -> Result<PathBuf> {
        let project_dirs = directories::ProjectDirs::from("", "", "rauncher").ok_or_else(|| {
            RcAuthError::InvalidResponse("Could not determine config directory".to_string())
        })?;

        Ok(project_dirs.config_dir().join("rc-auth"))
    }

    /// Get the path for an account file
    fn account_path(&self, account_key: &str) -> PathBuf {
        self.accounts_dir.join(format!("{}.json", account_key))
    }

    /// Acquire an exclusive lock on the storage
    async fn acquire_lock(&self) -> Result<std::fs::File> {
        let lock_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&self.lock_file)?;

        lock_file
            .try_lock_exclusive()
            .map_err(|_| RcAuthError::LockTimeout)?;

        Ok(lock_file)
    }

    /// Load and decrypt a session from disk
    async fn load_from_disk(&self, account_key: &str) -> Result<Option<Session>> {
        let path = self.account_path(account_key);

        if !path.exists() {
            return Ok(None);
        }

        // Read encrypted blob
        let content = fs::read_to_string(&path).await?;
        let encrypted: EncryptedBlob = serde_json::from_str(&content)
            .map_err(|e| RcAuthError::InvalidResponse(format!("Invalid encrypted data: {}", e)))?;

        // Decrypt
        let key_manager = self.key_manager.read().await;
        let plaintext = crypto::decrypt(key_manager.key(), &encrypted, account_key)?;

        // Deserialize session
        let session: Session = serde_json::from_slice(&plaintext)
            .map_err(|e| RcAuthError::InvalidResponse(format!("Invalid session data: {}", e)))?;

        Ok(Some(session))
    }

    /// Encrypt and save a session to disk
    async fn save_to_disk(&self, account_key: &str, session: &Session) -> Result<()> {
        let path = self.account_path(account_key);

        // Serialize session
        let plaintext = serde_json::to_vec(session).map_err(|e| {
            RcAuthError::InvalidResponse(format!("Failed to serialize session: {}", e))
        })?;

        // Encrypt
        let key_manager = self.key_manager.read().await;
        let encrypted = crypto::encrypt(key_manager.key(), &plaintext, account_key)?;

        // Serialize encrypted blob
        let encrypted_json = serde_json::to_string_pretty(&encrypted).map_err(|e| {
            RcAuthError::InvalidResponse(format!("Failed to serialize encrypted blob: {}", e))
        })?;

        // Atomic write: write to temp file, then rename
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, encrypted_json).await?;

        // Sync to disk
        let file = std::fs::File::open(&temp_path)?;
        file.sync_all()?;

        // Atomic rename
        fs::rename(&temp_path, &path).await?;

        // Set secure permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&path, perms)?;
        }

        Ok(())
    }

    /// Rotate encryption key and re-encrypt all sessions
    pub async fn rotate_key(&self) -> Result<()> {
        let _lock = self.acquire_lock().await?;

        // Load all sessions with current key
        let account_keys = self.list_accounts().await;
        let mut sessions = Vec::new();

        for key in &account_keys {
            if let Some(session) = self.load_from_disk(key).await? {
                sessions.push((key.clone(), session));
            }
        }

        // Rotate key
        let mut key_manager = self.key_manager.write().await;
        key_manager.rotate(&self.storage_dir).await?;
        drop(key_manager);

        // Re-encrypt all sessions with new key
        for (key, session) in sessions {
            self.save_to_disk(&key, &session).await?;
        }

        // Clear cache
        self.cache.write().await.clear();

        Ok(())
    }
}

#[async_trait::async_trait]
impl TokenStore for FileTokenStore {
    async fn load(&self, account_key: &str) -> Option<Session> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(session) = cache.get(account_key) {
                return Some(session.clone());
            }
        }

        // Load from disk
        match self.load_from_disk(account_key).await {
            Ok(Some(session)) => {
                // Update cache
                self.cache
                    .write()
                    .await
                    .insert(account_key.to_string(), session.clone());
                Some(session)
            }
            Ok(None) => None,
            Err(e) => {
                tracing::error!("Failed to load session for {}: {}", account_key, e);
                None
            }
        }
    }

    async fn save(&self, account_key: &str, session: &Session) -> Result<()> {
        let _lock = self.acquire_lock().await?;

        // Save to disk
        self.save_to_disk(account_key, session).await?;

        // Update cache
        self.cache
            .write()
            .await
            .insert(account_key.to_string(), session.clone());

        Ok(())
    }

    async fn remove(&self, account_key: &str) -> Result<()> {
        let _lock = self.acquire_lock().await?;

        let path = self.account_path(account_key);

        if path.exists() {
            fs::remove_file(&path).await?;
        }

        // Remove from cache
        self.cache.write().await.remove(account_key);

        Ok(())
    }

    async fn list_accounts(&self) -> Vec<String> {
        let mut accounts = Vec::new();

        let mut entries = match fs::read_dir(&self.accounts_dir).await {
            Ok(entries) => entries,
            Err(e) => {
                tracing::error!("Failed to read accounts directory: {}", e);
                return accounts;
            }
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                accounts.push(stem.to_string());
            }
        }

        accounts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret::StaticSecretProvider;
    use tempfile::TempDir;

    async fn create_test_store() -> (FileTokenStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let secret_provider = Arc::new(StaticSecretProvider::new("test-passphrase"));
        let store = FileTokenStore::new(temp_dir.path(), secret_provider)
            .await
            .unwrap();
        (store, temp_dir)
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let (store, _temp) = create_test_store().await;

        // Create a dummy session
        use crate::models::McProfile;
        use crate::session::*;

        let session = Session {
            ms: MsTokens::new("ms_token".to_string(), Some("refresh".to_string()), 3600),
            xbl: XblToken {
                token: "xbl_token".to_string(),
                uhs: "uhs".to_string(),
                not_after: None,
            },
            xsts: XstsToken {
                token: "xsts_token".to_string(),
                uhs: "uhs".to_string(),
                not_after: None,
            },
            mc: McToken::new("mc_token".to_string(), 3600),
            profile: McProfile {
                id: "test-uuid".to_string(),
                name: "TestPlayer".to_string(),
                skins: vec![],
                capes: vec![],
            },
            xuid: None,
            gamertag: None,
        };

        // Save
        store.save("test-uuid", &session).await.unwrap();

        // Load
        let loaded = store.load("test-uuid").await.unwrap();
        assert_eq!(loaded.profile.id, "test-uuid");
        assert_eq!(loaded.profile.name, "TestPlayer");
    }

    #[tokio::test]
    async fn test_remove() {
        let (store, _temp) = create_test_store().await;

        use crate::models::McProfile;
        use crate::session::*;

        let session = Session {
            ms: MsTokens::new("token".to_string(), None, 3600),
            xbl: XblToken {
                token: "xbl".to_string(),
                uhs: "uhs".to_string(),
                not_after: None,
            },
            xsts: XstsToken {
                token: "xsts".to_string(),
                uhs: "uhs".to_string(),
                not_after: None,
            },
            mc: McToken::new("mc".to_string(), 3600),
            profile: McProfile {
                id: "test-uuid".to_string(),
                name: "Test".to_string(),
                skins: vec![],
                capes: vec![],
            },
            xuid: None,
            gamertag: None,
        };

        store.save("test-uuid", &session).await.unwrap();
        assert!(store.load("test-uuid").await.is_some());

        store.remove("test-uuid").await.unwrap();
        assert!(store.load("test-uuid").await.is_none());
    }

    #[tokio::test]
    async fn test_list_accounts() {
        let (store, _temp) = create_test_store().await;

        use crate::models::McProfile;
        use crate::session::*;

        for i in 0..3 {
            let session = Session {
                ms: MsTokens::new("token".to_string(), None, 3600),
                xbl: XblToken {
                    token: "xbl".to_string(),
                    uhs: "uhs".to_string(),
                    not_after: None,
                },
                xsts: XstsToken {
                    token: "xsts".to_string(),
                    uhs: "uhs".to_string(),
                    not_after: None,
                },
                mc: McToken::new("mc".to_string(), 3600),
                profile: McProfile {
                    id: format!("uuid-{}", i),
                    name: format!("Player{}", i),
                    skins: vec![],
                    capes: vec![],
                },
                xuid: None,
                gamertag: None,
            };

            store.save(&format!("uuid-{}", i), &session).await.unwrap();
        }

        let accounts = store.list_accounts().await;
        assert_eq!(accounts.len(), 3);
    }
}
