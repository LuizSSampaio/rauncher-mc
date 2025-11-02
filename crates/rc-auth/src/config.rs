use std::time::Duration;
use url::Url;

/// Microsoft authentication endpoints
pub mod endpoints {
    pub const MS_AUTHORIZE: &str = "https://login.live.com/oauth20_authorize.srf";
    pub const MS_TOKEN: &str = "https://login.live.com/oauth20_token.srf";
    pub const XBL_AUTHENTICATE: &str = "https://user.auth.xboxlive.com/user/authenticate";
    pub const XSTS_AUTHORIZE: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
    pub const MC_LOGIN: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
    pub const MC_PROFILE: &str = "https://api.minecraftservices.com/minecraft/profile";
}

/// Official Minecraft launcher OAuth configuration
pub mod official {
    /// Official launcher client ID for development/testing
    pub const CLIENT_ID: &str = "00000000402B5328";
    pub const REDIRECT_URI: &str = "https://login.live.com/oauth20_desktop.srf";
    pub const SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";

    /// Additional query parameters for official flow
    pub const EXTRA_PARAMS: &[(&str, &str)] = &[
        ("lw", "1"),
        ("fl", "dob,easi2"),
        ("xsup", "1"),
        ("nopa", "2"),
    ];
}

/// Standard OAuth scope for custom apps
pub const STANDARD_SCOPE: &str = "XboxLive.signin offline_access";

/// Relying parties
pub const RP_MINECRAFT: &str = "rp://api.minecraftservices.com/";
pub const RP_XBOXLIVE: &str = "http://xboxlive.com";

/// Time skew for token expiration (refresh 5 minutes early)
pub const TOKEN_EXPIRY_SKEW: Duration = Duration::from_secs(300);

/// Authentication flow flavor
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizeFlavor {
    /// Official Minecraft launcher flow (recommended for development)
    /// Uses official client ID and doesn't require app approval
    OfficialDesktop,

    /// Standard OAuth2 code flow for custom approved apps
    /// Requires Mojang approval and custom client_id
    StandardCode,
}

impl Default for AuthorizeFlavor {
    fn default() -> Self {
        Self::OfficialDesktop
    }
}

/// HTTP client configuration
#[derive(Debug, Clone)]
pub struct HttpTimeouts {
    pub connect: Duration,
    pub request: Duration,
}

impl Default for HttpTimeouts {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(15),
            request: Duration::from_secs(30),
        }
    }
}

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(500),
        }
    }
}

/// Configuration for RcAuthClient
#[derive(Debug, Clone)]
pub struct RcAuthConfig {
    /// OAuth client ID (use official::CLIENT_ID for development)
    pub client_id: String,

    /// OAuth redirect URI
    pub redirect_uri: Url,

    /// Authorization flow flavor
    pub authorize_flavor: AuthorizeFlavor,

    /// HTTP client timeouts
    pub http_timeouts: HttpTimeouts,

    /// Custom user agent (optional)
    pub user_agent: Option<String>,

    /// Retry policy
    pub retry: RetryPolicy,
}

impl RcAuthConfig {
    /// Create config for official Minecraft launcher flow (for development)
    pub fn official_desktop() -> Self {
        Self {
            client_id: official::CLIENT_ID.to_string(),
            redirect_uri: Url::parse(official::REDIRECT_URI).expect("valid redirect URI"),
            authorize_flavor: AuthorizeFlavor::OfficialDesktop,
            http_timeouts: HttpTimeouts::default(),
            user_agent: Some("rauncher-mc".to_string()),
            retry: RetryPolicy::default(),
        }
    }

    /// Create config for custom approved app
    pub fn custom(client_id: String, redirect_uri: Url) -> Self {
        Self {
            client_id,
            redirect_uri,
            authorize_flavor: AuthorizeFlavor::StandardCode,
            http_timeouts: HttpTimeouts::default(),
            user_agent: Some("rauncher-mc".to_string()),
            retry: RetryPolicy::default(),
        }
    }
}

impl Default for RcAuthConfig {
    fn default() -> Self {
        Self::official_desktop()
    }
}
