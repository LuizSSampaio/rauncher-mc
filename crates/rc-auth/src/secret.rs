use zeroize::Zeroizing;

/// Trait for providing secrets (passphrases) for key derivation
/// 
/// Used as a fallback when OS keyring is unavailable or fails.
#[async_trait::async_trait]
pub trait SecretProvider: Send + Sync {
    /// Get a passphrase for key derivation
    /// 
    /// Returns None if the user cancels or no passphrase is available.
    /// The returned string will be automatically zeroized when dropped.
    async fn get_passphrase(&self, prompt: &str) -> Option<Zeroizing<String>>;
}

/// No-op secret provider that always returns None
/// 
/// Use this when you want keyring-only authentication with no passphrase fallback.
#[derive(Debug, Clone, Default)]
pub struct NoSecretProvider;

#[async_trait::async_trait]
impl SecretProvider for NoSecretProvider {
    async fn get_passphrase(&self, _prompt: &str) -> Option<Zeroizing<String>> {
        None
    }
}

/// Static secret provider for testing
#[derive(Debug, Clone)]
pub struct StaticSecretProvider {
    secret: String,
}

impl StaticSecretProvider {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
        }
    }
}

#[async_trait::async_trait]
impl SecretProvider for StaticSecretProvider {
    async fn get_passphrase(&self, _prompt: &str) -> Option<Zeroizing<String>> {
        Some(Zeroizing::new(self.secret.clone()))
    }
}
