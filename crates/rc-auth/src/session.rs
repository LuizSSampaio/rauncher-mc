use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::McProfile;

/// Complete authentication session with all tokens and profile
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub ms: MsTokens,
    pub xbl: XblToken,
    pub xsts: XstsToken,
    pub mc: McToken,
    pub profile: McProfile,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gamertag: Option<String>,
}

impl Session {
    /// Check if the Minecraft access token needs refresh
    pub fn needs_refresh(&self) -> bool {
        self.mc.is_expired()
    }
    
    /// Get the account key (UUID) for storage
    pub fn account_key(&self) -> &str {
        &self.profile.id
    }
}

/// Microsoft OAuth tokens
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MsTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
}

impl MsTokens {
    pub fn new(access_token: String, refresh_token: Option<String>, expires_in: u64) -> Self {
        let expires_at = Utc::now() + chrono::Duration::seconds(expires_in as i64);
        Self {
            access_token,
            refresh_token,
            expires_at,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }
}

/// Xbox Live token
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct XblToken {
    pub token: String,
    pub uhs: String,
    pub not_after: Option<String>,
}

/// XSTS token
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct XstsToken {
    pub token: String,
    pub uhs: String,
    pub not_after: Option<String>,
}

/// Minecraft access token
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McToken {
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
}

impl McToken {
    pub fn new(access_token: String, expires_in: u64) -> Self {
        let expires_at = Utc::now() + chrono::Duration::seconds(expires_in as i64);
        Self {
            access_token,
            expires_at,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        use crate::config::TOKEN_EXPIRY_SKEW;
        let skew_duration = chrono::Duration::from_std(TOKEN_EXPIRY_SKEW)
            .unwrap_or(chrono::Duration::seconds(300));
        Utc::now() + skew_duration >= self.expires_at
    }
}
