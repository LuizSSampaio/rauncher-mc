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
//! use rc_auth::{RcAuthClient, RcAuthConfig};
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
//!     let code = client.parse_redirect(&redirect_url, None)?;
//!     
//!     // Complete the login flow
//!     let session = client.complete_login_with_code(&code).await?;
//!     println!("Logged in as: {}", session.profile.name);
//!     
//!     // Later, refresh the session when needed
//!     if session.needs_refresh() {
//!         let refreshed = client.refresh_session(&session).await?;
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
//! ```no_run
//! use rc_auth::{MemoryTokenStore, TokenStore};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let store = MemoryTokenStore::new();
//!
//! // Save session
//! # let session = todo!();
//! store.save(session.account_key(), &session).await?;
//!
//! // Load session later
//! if let Some(session) = store.load("account_id").await {
//!     println!("Loaded session for: {}", session.profile.name);
//! }
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
pub mod errors;
pub mod models;
pub mod session;
pub mod store;

// Re-export main types
pub use client::RcAuthClient;
pub use config::{AuthorizeFlavor, RcAuthConfig};
pub use errors::{RcAuthError, Result, XstsError};
pub use models::McProfile;
pub use session::{McToken, MsTokens, Session, XblToken, XstsToken};
pub use store::{MemoryTokenStore, TokenStore};
