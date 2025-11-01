use thiserror::Error;

/// Microsoft Authentication Scheme error types
#[derive(Error, Debug)]
pub enum RcAuthError {
    #[error("User cancelled the authentication flow")]
    UserCancelled,

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("HTTP error {status}: {body_snippet}")]
    Http {
        status: reqwest::StatusCode,
        body_snippet: String,
    },

    #[error("OAuth invalid_grant - refresh token may be expired")]
    OAuthInvalidGrant,

    #[error("Xbox Live authentication failed after retry")]
    XblBadRequest,

    #[error("XSTS authorization denied: {0}")]
    XstsDenied(#[from] XstsError),

    #[error("Minecraft profile not found - user may not own Minecraft or hasn't created a profile")]
    MinecraftProfileNotFound,

    #[error("Invalid redirect URI or missing code")]
    InvalidRedirect,

    #[error("OAuth state mismatch - possible CSRF attack")]
    StateMismatch,

    #[error("JSON serialization/deserialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("Missing refresh token - cannot refresh session")]
    MissingRefreshToken,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Storage I/O error: {0}")]
    StorageIo(#[from] std::io::Error),

    #[error("Cryptography error: {0}")]
    Crypto(String),

    #[error("Keyring error: {0}")]
    Keyring(String),

    #[error("Corrupted storage - decryption or integrity check failed")]
    CorruptedStore,

    #[error("Lock timeout - another process may be using the storage")]
    LockTimeout,

    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
}

/// XSTS-specific error codes from XErr field
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum XstsError {
    #[error("Account doesn't have an Xbox account (XErr: 2148916233)")]
    NoXboxAccount,

    #[error("Xbox Live not available in this country (XErr: 2148916235)")]
    RegionNotSupported,

    #[error("Adult verification required on Xbox page (XErr: 2148916236/2148916237)")]
    AdultVerificationRequired,

    #[error("Child account requires Family (XErr: 2148916238)")]
    ChildAccountRequiresFamily,

    #[error("Unknown XSTS error code: {0}")]
    Unknown(u64),
}

impl XstsError {
    /// Parse XErr code from XSTS response
    pub fn from_xerr(code: u64) -> Self {
        match code {
            2148916233 => Self::NoXboxAccount,
            2148916235 => Self::RegionNotSupported,
            2148916236 | 2148916237 => Self::AdultVerificationRequired,
            2148916238 => Self::ChildAccountRequiresFamily,
            code => Self::Unknown(code),
        }
    }
}

pub type Result<T> = std::result::Result<T, RcAuthError>;
