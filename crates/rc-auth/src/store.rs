use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::errors::Result;
use crate::session::Session;

/// Trait for storing and retrieving authentication sessions
#[async_trait::async_trait]
pub trait TokenStore: Send + Sync {
    /// Load a session by account key (UUID)
    async fn load(&self, account_key: &str) -> Option<Session>;

    /// Save a session by account key (UUID)
    async fn save(&self, account_key: &str, session: &Session) -> Result<()>;

    /// Remove a session by account key (UUID)
    async fn remove(&self, account_key: &str) -> Result<()>;

    /// List all stored account keys
    async fn list_accounts(&self) -> Vec<String>;
}

/// In-memory token store for testing and simple use cases
#[derive(Debug, Clone, Default)]
pub struct MemoryTokenStore {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl MemoryTokenStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl TokenStore for MemoryTokenStore {
    async fn load(&self, account_key: &str) -> Option<Session> {
        self.sessions.read().ok()?.get(account_key).cloned()
    }

    async fn save(&self, account_key: &str, session: &Session) -> Result<()> {
        self.sessions
            .write()
            .map_err(|_| crate::errors::RcAuthError::InvalidResponse("Lock poisoned".to_string()))?
            .insert(account_key.to_string(), session.clone());
        Ok(())
    }

    async fn remove(&self, account_key: &str) -> Result<()> {
        self.sessions
            .write()
            .map_err(|_| crate::errors::RcAuthError::InvalidResponse("Lock poisoned".to_string()))?
            .remove(account_key);
        Ok(())
    }

    async fn list_accounts(&self) -> Vec<String> {
        self.sessions
            .read()
            .ok()
            .map(|s| s.keys().cloned().collect())
            .unwrap_or_default()
    }
}
