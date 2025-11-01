use reqwest::{Client, StatusCode};
use tracing::{debug, instrument, warn};
use url::Url;

use crate::config::{endpoints, official, AuthorizeFlavor, RcAuthConfig, RP_MINECRAFT, RP_XBOXLIVE, STANDARD_SCOPE};
use crate::errors::{RcAuthError, Result, XstsError};
use crate::models::*;
use crate::session::{McToken, MsTokens, Session, XblToken, XstsToken};

/// Main client for Microsoft authentication
#[derive(Debug, Clone)]
pub struct RcAuthClient {
    config: RcAuthConfig,
    http: Client,
}

impl RcAuthClient {
    /// Create a new authentication client
    pub fn new(config: RcAuthConfig) -> Result<Self> {
        let http = Client::builder()
            .connect_timeout(config.http_timeouts.connect)
            .timeout(config.http_timeouts.request)
            .user_agent(config.user_agent.as_deref().unwrap_or("rauncher-mc"))
            .build()?;
        
        Ok(Self { config, http })
    }
    
    /// Build the authorization URL for the user to visit
    #[instrument(skip(self))]
    pub fn build_authorize_url(&self, state: Option<String>) -> Result<Url> {
        let mut url = Url::parse(endpoints::MS_AUTHORIZE)?;
        
        match &self.config.authorize_flavor {
            AuthorizeFlavor::OfficialDesktop => {
                url.query_pairs_mut()
                    .append_pair("client_id", &self.config.client_id)
                    .append_pair("response_type", "code")
                    .append_pair("redirect_uri", self.config.redirect_uri.as_str())
                    .append_pair("scope", official::SCOPE)
                    .append_pair("prompt", "select_account");
                
                for (key, value) in official::EXTRA_PARAMS {
                    url.query_pairs_mut().append_pair(key, value);
                }
                
                if let Some(s) = state {
                    url.query_pairs_mut().append_pair("state", &s);
                }
            }
            AuthorizeFlavor::StandardCode => {
                url.query_pairs_mut()
                    .append_pair("client_id", &self.config.client_id)
                    .append_pair("response_type", "code")
                    .append_pair("redirect_uri", self.config.redirect_uri.as_str())
                    .append_pair("scope", STANDARD_SCOPE)
                    .append_pair("prompt", "select_account");
                
                if let Some(s) = state {
                    url.query_pairs_mut().append_pair("state", &s);
                }
            }
        }
        
        debug!("Built authorize URL: {}", url);
        Ok(url)
    }
    
    /// Parse the redirect URL and extract the authorization code
    #[instrument(skip(self))]
    pub fn parse_redirect(&self, redirect_url: &str, expected_state: Option<&str>) -> Result<String> {
        let url = Url::parse(redirect_url)?;
        let params: std::collections::HashMap<_, _> = url.query_pairs().collect();
        
        if let Some(error) = params.get("error") {
            if error == "access_denied" {
                return Err(RcAuthError::UserCancelled);
            }
            return Err(RcAuthError::InvalidRedirect);
        }
        
        if let Some(expected) = expected_state {
            match params.get("state") {
                Some(actual) if actual == expected => {}
                _ => return Err(RcAuthError::StateMismatch),
            }
        }
        
        params
            .get("code")
            .map(|c| c.to_string())
            .ok_or(RcAuthError::InvalidRedirect)
    }
    
    /// Exchange authorization code for Microsoft tokens
    #[instrument(skip(self, code))]
    pub async fn exchange_code(&self, code: &str) -> Result<MsTokens> {
        let scope = match &self.config.authorize_flavor {
            AuthorizeFlavor::OfficialDesktop => official::SCOPE,
            AuthorizeFlavor::StandardCode => STANDARD_SCOPE,
        };
        
        let mut url = Url::parse(endpoints::MS_TOKEN)?;
        url.query_pairs_mut()
            .append_pair("client_id", &self.config.client_id)
            .append_pair("code", code)
            .append_pair("redirect_uri", self.config.redirect_uri.as_str())
            .append_pair("grant_type", "authorization_code")
            .append_pair("scope", scope);
        
        debug!("Exchanging authorization code for tokens");
        let response = self.http.get(url).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            
            if body.contains("invalid_grant") {
                return Err(RcAuthError::OAuthInvalidGrant);
            }
            
            return Err(RcAuthError::Http {
                status,
                body_snippet: body.chars().take(200).collect(),
            });
        }
        
        let token_response: MsTokenResponse = response.json().await?;
        Ok(MsTokens::new(
            token_response.access_token,
            token_response.refresh_token,
            token_response.expires_in,
        ))
    }
    
    /// Refresh Microsoft tokens using refresh_token
    #[instrument(skip(self, refresh_token))]
    pub async fn refresh_ms_token(&self, refresh_token: &str) -> Result<MsTokens> {
        let scope = match &self.config.authorize_flavor {
            AuthorizeFlavor::OfficialDesktop => official::SCOPE,
            AuthorizeFlavor::StandardCode => STANDARD_SCOPE,
        };
        
        let mut url = Url::parse(endpoints::MS_TOKEN)?;
        url.query_pairs_mut()
            .append_pair("client_id", &self.config.client_id)
            .append_pair("refresh_token", refresh_token)
            .append_pair("grant_type", "refresh_token")
            .append_pair("scope", scope);
        
        debug!("Refreshing Microsoft access token");
        let response = self.http.get(url).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            
            if body.contains("invalid_grant") {
                return Err(RcAuthError::OAuthInvalidGrant);
            }
            
            return Err(RcAuthError::Http {
                status,
                body_snippet: body.chars().take(200).collect(),
            });
        }
        
        let token_response: MsTokenResponse = response.json().await?;
        Ok(MsTokens::new(
            token_response.access_token,
            token_response.refresh_token,
            token_response.expires_in,
        ))
    }
    
    /// Authenticate with Xbox Live
    #[instrument(skip(self, ms_access_token))]
    pub async fn xbl_authenticate(&self, ms_access_token: &str) -> Result<XblToken> {
        let request = XblAuthRequest {
            properties: XblAuthProperties {
                auth_method: "RPS".to_string(),
                site_name: "user.auth.xboxlive.com".to_string(),
                rps_ticket: ms_access_token.to_string(),
            },
            relying_party: "http://auth.xboxlive.com".to_string(),
            token_type: "JWT".to_string(),
        };
        
        debug!("Authenticating with Xbox Live");
        let response = self.http
            .post(endpoints::XBL_AUTHENTICATE)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;
        
        // Handle the "d=" retry caveat
        if response.status() == StatusCode::BAD_REQUEST {
            warn!("XBL authentication failed, retrying with 'd=' prefix");
            
            let retry_request = XblAuthRequest {
                properties: XblAuthProperties {
                    auth_method: "RPS".to_string(),
                    site_name: "user.auth.xboxlive.com".to_string(),
                    rps_ticket: format!("d={}", ms_access_token),
                },
                relying_party: "http://auth.xboxlive.com".to_string(),
                token_type: "JWT".to_string(),
            };
            
            let retry_response = self.http
                .post(endpoints::XBL_AUTHENTICATE)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .json(&retry_request)
                .send()
                .await?;
            
            if !retry_response.status().is_success() {
                return Err(RcAuthError::XblBadRequest);
            }
            
            let xbl_response: XblAuthResponse = retry_response.json().await?;
            let uhs = xbl_response
                .display_claims
                .xui
                .first()
                .ok_or_else(|| RcAuthError::InvalidResponse("Missing XUI claims".to_string()))?
                .uhs
                .clone();
            
            return Ok(XblToken {
                token: xbl_response.token,
                uhs,
                not_after: xbl_response.not_after,
            });
        }
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(RcAuthError::Http {
                status,
                body_snippet: body.chars().take(200).collect(),
            });
        }
        
        let xbl_response: XblAuthResponse = response.json().await?;
        let uhs = xbl_response
            .display_claims
            .xui
            .first()
            .ok_or_else(|| RcAuthError::InvalidResponse("Missing XUI claims".to_string()))?
            .uhs
            .clone();
        
        Ok(XblToken {
            token: xbl_response.token,
            uhs,
            not_after: xbl_response.not_after,
        })
    }
    
    /// Authorize with XSTS
    #[instrument(skip(self, xbl_token))]
    pub async fn xsts_authorize(&self, xbl_token: &str) -> Result<XstsToken> {
        let request = XstsAuthRequest {
            properties: XstsAuthProperties {
                sandbox_id: "RETAIL".to_string(),
                user_tokens: vec![xbl_token.to_string()],
                optional_display_claims: None,
            },
            relying_party: RP_MINECRAFT.to_string(),
            token_type: "JWT".to_string(),
        };
        
        debug!("Authorizing with XSTS");
        let response = self.http
            .post(endpoints::XSTS_AUTHORIZE)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;
        
        if response.status() == StatusCode::UNAUTHORIZED {
            let error_response: XstsErrorResponse = response.json().await?;
            return Err(XstsError::from_xerr(error_response.xerr).into());
        }
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(RcAuthError::Http {
                status,
                body_snippet: body.chars().take(200).collect(),
            });
        }
        
        let xsts_response: XstsAuthResponse = response.json().await?;
        let uhs = xsts_response
            .display_claims
            .xui
            .first()
            .ok_or_else(|| RcAuthError::InvalidResponse("Missing XUI claims".to_string()))?
            .uhs
            .clone();
        
        Ok(XstsToken {
            token: xsts_response.token,
            uhs,
            not_after: xsts_response.not_after,
        })
    }
    
    /// Fetch XUID and gamertag (optional)
    #[instrument(skip(self, xbl_token))]
    pub async fn fetch_xuid(&self, xbl_token: &str) -> Result<(String, String)> {
        let request = XstsAuthRequest {
            properties: XstsAuthProperties {
                sandbox_id: "RETAIL".to_string(),
                user_tokens: vec![xbl_token.to_string()],
                optional_display_claims: Some(vec!["mgt".to_string(), "mgs".to_string(), "umg".to_string()]),
            },
            relying_party: RP_XBOXLIVE.to_string(),
            token_type: "JWT".to_string(),
        };
        
        debug!("Fetching XUID and gamertag");
        let response = self.http
            .post(endpoints::XSTS_AUTHORIZE)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(RcAuthError::Http {
                status,
                body_snippet: body.chars().take(200).collect(),
            });
        }
        
        let xsts_response: XstsAuthResponse = response.json().await?;
        let user_info = xsts_response
            .display_claims
            .xui
            .first()
            .ok_or_else(|| RcAuthError::InvalidResponse("Missing XUI claims".to_string()))?;
        
        let xuid = user_info
            .xid
            .clone()
            .ok_or_else(|| RcAuthError::InvalidResponse("Missing XUID".to_string()))?;
        let gamertag = user_info
            .gtg
            .clone()
            .ok_or_else(|| RcAuthError::InvalidResponse("Missing gamertag".to_string()))?;
        
        Ok((xuid, gamertag))
    }
    
    /// Login to Minecraft with XSTS token
    #[instrument(skip(self, xsts_token, uhs))]
    pub async fn mc_login(&self, xsts_token: &str, uhs: &str) -> Result<McToken> {
        let identity_token = format!("XBL3.0 x={};{}", uhs, xsts_token);
        let request = McLoginRequest { identity_token };
        
        debug!("Logging in to Minecraft Services");
        let response = self.http
            .post(endpoints::MC_LOGIN)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(RcAuthError::Http {
                status,
                body_snippet: body.chars().take(200).collect(),
            });
        }
        
        let mc_response: McLoginResponse = response.json().await?;
        Ok(McToken::new(mc_response.access_token, mc_response.expires_in))
    }
    
    /// Fetch Minecraft profile
    #[instrument(skip(self, mc_access_token))]
    pub async fn fetch_profile(&self, mc_access_token: &str) -> Result<McProfile> {
        debug!("Fetching Minecraft profile");
        let response = self.http
            .get(endpoints::MC_PROFILE)
            .header("Authorization", format!("Bearer {}", mc_access_token))
            .send()
            .await?;
        
        let status = response.status();
        
        // Handle NOT_FOUND specifically for Minecraft profile
        if status == StatusCode::NOT_FOUND {
            return Err(RcAuthError::MinecraftProfileNotFound);
        }
        
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(RcAuthError::Http {
                status,
                body_snippet: body.chars().take(200).collect(),
            });
        }
        
        let profile: McProfile = response.json().await?;
        Ok(profile)
    }
    
    /// Complete login flow from authorization code to full session
    #[instrument(skip(self, code))]
    pub async fn complete_login_with_code(&self, code: &str) -> Result<Session> {
        debug!("Starting complete login flow");
        
        // Step 1: Exchange code for MS tokens
        let ms = self.exchange_code(code).await?;
        
        // Step 2: Authenticate with Xbox Live
        let xbl = self.xbl_authenticate(&ms.access_token).await?;
        
        // Step 3: Authorize with XSTS
        let xsts = self.xsts_authorize(&xbl.token).await?;
        
        // Step 4: Login to Minecraft
        let mc = self.mc_login(&xsts.token, &xsts.uhs).await?;
        
        // Step 5: Fetch profile
        let profile = self.fetch_profile(&mc.access_token).await?;
        
        // Step 6 (optional): Fetch XUID and gamertag
        let (xuid, gamertag) = match self.fetch_xuid(&xbl.token).await {
            Ok((x, g)) => (Some(x), Some(g)),
            Err(e) => {
                warn!("Failed to fetch XUID/gamertag: {}", e);
                (None, None)
            }
        };
        
        Ok(Session {
            ms,
            xbl,
            xsts,
            mc,
            profile,
            xuid,
            gamertag,
        })
    }
    
    /// Refresh an existing session
    #[instrument(skip(self, session))]
    pub async fn refresh_session(&self, session: &Session) -> Result<Session> {
        debug!("Refreshing session");
        
        // Step 1: Refresh MS token
        let refresh_token = session
            .ms
            .refresh_token
            .as_ref()
            .ok_or(RcAuthError::MissingRefreshToken)?;
        
        let ms = self.refresh_ms_token(refresh_token).await?;
        
        // Step 2: Re-authenticate through the chain
        let xbl = self.xbl_authenticate(&ms.access_token).await?;
        let xsts = self.xsts_authorize(&xbl.token).await?;
        let mc = self.mc_login(&xsts.token, &xsts.uhs).await?;
        
        // Keep the same profile and XUID/gamertag
        Ok(Session {
            ms,
            xbl,
            xsts,
            mc,
            profile: session.profile.clone(),
            xuid: session.xuid.clone(),
            gamertag: session.gamertag.clone(),
        })
    }
}
