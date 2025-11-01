use serde::{Deserialize, Serialize};

/// Microsoft OAuth token response (from both code and refresh_token grants)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsTokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
    pub token_type: String,
    #[serde(default)]
    pub scope: Option<String>,
}

/// Xbox Live user.authenticate request
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct XblAuthRequest {
    pub properties: XblAuthProperties,
    pub relying_party: String,
    pub token_type: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct XblAuthProperties {
    pub auth_method: String,
    pub site_name: String,
    pub rps_ticket: String,
}

/// Xbox Live user.authenticate response
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct XblAuthResponse {
    pub token: String,
    pub display_claims: XblDisplayClaims,
    #[serde(default)]
    pub issue_instant: Option<String>,
    #[serde(default)]
    pub not_after: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct XblDisplayClaims {
    pub xui: Vec<XblUserInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct XblUserInfo {
    pub uhs: String,
    #[serde(default)]
    pub xid: Option<String>,
    #[serde(default)]
    pub gtg: Option<String>,
}

/// XSTS authorize request
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct XstsAuthRequest {
    pub properties: XstsAuthProperties,
    pub relying_party: String,
    pub token_type: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct XstsAuthProperties {
    pub sandbox_id: String,
    pub user_tokens: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_display_claims: Option<Vec<String>>,
}

/// XSTS authorize response
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct XstsAuthResponse {
    pub token: String,
    pub display_claims: XblDisplayClaims,
    #[serde(default)]
    pub issue_instant: Option<String>,
    #[serde(default)]
    pub not_after: Option<String>,
}

/// XSTS error response
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct XstsErrorResponse {
    #[serde(rename = "XErr")]
    pub xerr: u64,
    #[serde(default)]
    pub message: Option<String>,
}

/// Minecraft login_with_xbox request
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McLoginRequest {
    pub identity_token: String,
}

/// Minecraft login_with_xbox response
#[derive(Debug, Clone, Deserialize)]
pub struct McLoginResponse {
    pub username: String,
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

/// Minecraft profile response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McProfile {
    /// UUID without dashes
    pub id: String,
    /// Player name
    pub name: String,
    #[serde(default)]
    pub skins: Vec<McSkin>,
    #[serde(default)]
    pub capes: Vec<McCape>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McSkin {
    pub id: String,
    pub state: String,
    pub url: String,
    pub variant: String,
    #[serde(default)]
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McCape {
    pub id: String,
    pub state: String,
    pub url: String,
    #[serde(default)]
    pub alias: Option<String>,
}

/// Minecraft profile error response
#[derive(Debug, Clone, Deserialize)]
pub struct McProfileError {
    pub error: String,
    #[serde(default)]
    pub error_message: Option<String>,
}
