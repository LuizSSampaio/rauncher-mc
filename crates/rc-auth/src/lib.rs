//! Microsoft Authentication Scheme implementation for Minecraft launchers
//!
//! This crate provides a complete implementation of the Microsoft Authentication Scheme
//! used by Minecraft launchers to authenticate users via Microsoft accounts.
//!
//! # Authentication Flow
//!
//! The authentication flow consists of several steps:
//!
//! 1. OAuth2 authorization with Microsoft
//! 2. Xbox Live authentication
//! 3. XSTS authorization
//! 4. Minecraft Services login
//! 5. Profile retrieval
//!
//! # Example
//!
//! ```no_run
//! use rc-auth::{RcAuthClient, RcAuthConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create client with official desktop flow (for development)
//!     let config = RcAuthConfig::official_desktop();
//!     let client = RcAuthClient::new(config)?;
//!     
//!     // Build authorization URL for user to visit
//!     let auth_url = client.build_authorize_url(None)?;
//!     println!("Visit: {}", auth_url);
//!     
//!     // After user authorizes and you receive the redirect URL with code...
//!     let redirect_url = "http://localhost:8000/?code=..."; // From user
//!     let code = client.parse_redirect(redirect_url, None)?;
//!     
//!     // Complete the login flow
//!     let session = client.complete_login_with_code(&code).await?;
//!     println!("Logged in as: {}", session.profile.name);
//!     
//!     // Later, refresh the session when needed
//!     if session.needs_refresh() {
//!         let _refreshed = client.refresh_session(&session).await?;
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! # Token Storage
//!
//! The crate provides a `TokenStore` trait for persisting sessions:
//!
//! ## In-Memory Storage (Testing)
//!
//! ```
//! use rc-auth::{MemoryTokenStore, TokenStore};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let store = MemoryTokenStore::new();
//!
//! // Save session (create a mock session for example)
//! # use rc-auth::{Session, MsTokens, XblToken, XstsToken, McToken, McProfile};
//! # let session = Session {
//! #     ms: MsTokens::new("token".to_string(), None, 3600),
//! #     xbl: XblToken { token: "xbl".to_string(), uhs: "uhs".to_string(), not_after: None },
//! #     xsts: XstsToken { token: "xsts".to_string(), uhs: "uhs".to_string(), not_after: None },
//! #     mc: McToken::new("mc".to_string(), 3600),
//! #     profile: McProfile { id: "uuid".to_string(), name: "Player".to_string(), skins: vec![], capes: vec![] },
//! #     xuid: None,
//! #     gamertag: None,
//! # };
//! store.save(session.account_key(), &session).await?;
//!
//! // Load session later
//! if let Some(session) = store.load("uuid").await {
//!     println!("Loaded session for: {}", session.profile.name);
//! }
//! # Ok(())
//! # }
//! # tokio_test::block_on(example());
//! ```
//!
//! ## File-Based Encrypted Storage (Production)
//!
//! ```no_run
//! use rc-auth::{FileTokenStore, NoSecretProvider, TokenStore};
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Use OS keyring for key storage (no passphrase needed)
//! let secret_provider = Arc::new(NoSecretProvider);
//! let storage_dir = FileTokenStore::default_storage_dir()?;
//! let store = FileTokenStore::new(storage_dir, secret_provider).await?;
//!
//! // Save session (encrypted automatically)
//! # use rc-auth::{Session, MsTokens, XblToken, XstsToken, McToken, McProfile};
//! # let session = Session {
//! #     ms: MsTokens::new("token".to_string(), None, 3600),
//! #     xbl: XblToken { token: "xbl".to_string(), uhs: "uhs".to_string(), not_after: None },
//! #     xsts: XstsToken { token: "xsts".to_string(), uhs: "uhs".to_string(), not_after: None },
//! #     mc: McToken::new("mc".to_string(), 3600),
//! #     profile: McProfile { id: "uuid".to_string(), name: "Player".to_string(), skins: vec![], capes: vec![] },
//! #     xuid: None,
//! #     gamertag: None,
//! # };
//! store.save(session.account_key(), &session).await?;
//!
//! // Sessions are encrypted using AES-256-GCM
//! // Keys are stored in OS keyring (macOS Keychain, Windows Credential Manager, Linux Secret Service)
//! # Ok(())
//! # }
//! ```
//!
//! # Important Notes
//!
//! - For development, use `RcAuthConfig::official_desktop()` with the official launcher's client ID
//! - For production, you need Mojang approval and your own client ID
//! - Tokens should be stored securely and never logged
//! - The MC access token expires after 24 hours and needs refresh

pub mod client;
pub mod config;
pub mod crypto;
pub mod errors;
pub mod file_store;
pub mod key_manager;
pub mod models;
pub mod secret;
pub mod session;
pub mod store;

// Re-export main types
pub use client::RcAuthClient;
pub use config::{AuthorizeFlavor, RcAuthConfig};
pub use errors::{RcAuthError, Result, XstsError};
pub use file_store::FileTokenStore;
pub use models::McProfile;
pub use secret::{NoSecretProvider, SecretProvider, StaticSecretProvider};
pub use session::{McToken, MsTokens, Session, XblToken, XstsToken};
pub use store::{MemoryTokenStore, TokenStore};
