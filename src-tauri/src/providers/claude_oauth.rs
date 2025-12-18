//! Claude OAuth Provider
//!
//! 实现 Anthropic Claude OAuth 认证流程，与 CLIProxyAPI 对齐。
//! 支持 Token 刷新、重试机制和统一凭证格式。

use super::error::{
    create_auth_error, create_config_error, create_token_refresh_error, ProviderError,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;

// OAuth 端点和凭证 - 与 CLIProxyAPI 完全一致
const CLAUDE_AUTH_URL: &str = "https://claude.ai/oauth/authorize";
const CLAUDE_TOKEN_URL: &str = "https://console.anthropic.com/v1/oauth/token";
const CLAUDE_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const DEFAULT_CALLBACK_PORT: u16 = 54545;

/// Claude OAuth 凭证存储
///
/// 与 CLIProxyAPI 的 ClaudeTokenStorage 格式兼容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeOAuthCredentials {
    /// 访问令牌
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    /// 刷新令牌
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// 用户邮箱
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// 过期时间（RFC3339 格式）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expire: Option<String>,
    /// 最后刷新时间（RFC3339 格式）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_refresh: Option<String>,
    /// 凭证类型标识
    #[serde(default = "default_claude_type", rename = "type")]
    pub cred_type: String,
}

fn default_claude_type() -> String {
    "claude_oauth".to_string()
}

impl Default for ClaudeOAuthCredentials {
    fn default() -> Self {
        Self {
            access_token: None,
            refresh_token: None,
            email: None,
            expire: None,
            last_refresh: None,
            cred_type: default_claude_type(),
        }
    }
}

/// PKCE codes for OAuth2 authorization
#[derive(Debug, Clone)]
pub struct PKCECodes {
    /// Cryptographically random string for code verification
    pub code_verifier: String,
    /// SHA256 hash of code_verifier, base64url-encoded
    pub code_challenge: String,
}

impl PKCECodes {
    /// Generate new PKCE codes
    pub fn generate() -> Result<Self, Box<dyn Error + Send + Sync>> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        use rand::RngCore;
        use sha2::{Digest, Sha256};

        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        let code_verifier = URL_SAFE_NO_PAD.encode(bytes);

        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let hash = hasher.finalize();
        let code_challenge = URL_SAFE_NO_PAD.encode(hash);

        Ok(Self {
            code_verifier,
            code_challenge,
        })
    }
}

/// Claude OAuth Provider
///
/// 处理 Anthropic Claude 的 OAuth 认证和 API 调用
pub struct ClaudeOAuthProvider {
    /// OAuth 凭证
    pub credentials: ClaudeOAuthCredentials,
    /// HTTP 客户端
    pub client: Client,
    /// 凭证文件路径
    pub creds_path: Option<PathBuf>,
    /// OAuth 回调端口
    pub callback_port: u16,
}

impl Default for ClaudeOAuthProvider {
    fn default() -> Self {
        Self {
            credentials: ClaudeOAuthCredentials::default(),
            client: Client::new(),
            creds_path: None,
            callback_port: DEFAULT_CALLBACK_PORT,
        }
    }
}

impl ClaudeOAuthProvider {
    /// 创建新的 ClaudeOAuthProvider 实例
    pub fn new() -> Self {
        Self::default()
    }

    /// 使用自定义 HTTP 客户端创建
    pub fn with_client(client: Client) -> Self {
        Self {
            client,
            ..Self::default()
        }
    }

    /// 获取默认凭证文件路径
    pub fn default_creds_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".claude")
            .join("oauth_creds.json")
    }

    /// 从默认路径加载凭证
    pub async fn load_credentials(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let path = Self::default_creds_path();
        self.load_credentials_from_path_internal(&path).await
    }

    /// 从指定路径加载凭证
    pub async fn load_credentials_from_path(
        &mut self,
        path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let path = PathBuf::from(path);
        self.load_credentials_from_path_internal(&path).await
    }

    async fn load_credentials_from_path_internal(
        &mut self,
        path: &PathBuf,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            let content = tokio::fs::read_to_string(&path).await?;
            let creds: ClaudeOAuthCredentials = serde_json::from_str(&content)?;
            tracing::info!(
                "[CLAUDE_OAUTH] 凭证已加载: has_access={}, has_refresh={}, email={:?}",
                creds.access_token.is_some(),
                creds.refresh_token.is_some(),
                creds.email
            );
            self.credentials = creds;
            self.creds_path = Some(path.clone());
        } else {
            tracing::warn!("[CLAUDE_OAUTH] 凭证文件不存在: {:?}", path);
        }
        Ok(())
    }

    /// 保存凭证到文件
    pub async fn save_credentials(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let path = self
            .creds_path
            .clone()
            .unwrap_or_else(Self::default_creds_path);

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(&self.credentials)?;
        tokio::fs::write(&path, content).await?;
        tracing::info!("[CLAUDE_OAUTH] 凭证已保存到 {:?}", path);
        Ok(())
    }

    /// 检查 Token 是否有效
    pub fn is_token_valid(&self) -> bool {
        if self.credentials.access_token.is_none() {
            return false;
        }

        if let Some(expire_str) = &self.credentials.expire {
            if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(expire_str) {
                let now = chrono::Utc::now();
                return expires > now + chrono::Duration::minutes(5);
            }
        }

        true
    }

    /// 刷新 Token - 与 CLIProxyAPI 对齐，使用 JSON 格式
    pub async fn refresh_token(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        let refresh_token = self
            .credentials
            .refresh_token
            .as_ref()
            .ok_or_else(|| create_config_error("没有可用的 refresh_token"))?;

        tracing::info!("[CLAUDE_OAUTH] 正在刷新 Token");

        // 与 CLIProxyAPI 对齐：使用 JSON 格式请求体
        let body = serde_json::json!({
            "client_id": CLAUDE_CLIENT_ID,
            "grant_type": "refresh_token",
            "refresh_token": refresh_token
        });

        let resp = self
            .client
            .post(CLAUDE_TOKEN_URL)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Box::new(ProviderError::from(e)) as Box<dyn Error + Send + Sync>)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!("[CLAUDE_OAUTH] Token 刷新失败: {} - {}", status, body);
            self.mark_invalid();
            return Err(create_token_refresh_error(status, &body, "CLAUDE_OAUTH"));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| Box::new(ProviderError::from(e)) as Box<dyn Error + Send + Sync>)?;

        let new_access_token = data["access_token"]
            .as_str()
            .ok_or_else(|| create_auth_error("响应中没有 access_token"))?
            .to_string();

        self.credentials.access_token = Some(new_access_token.clone());

        if let Some(rt) = data["refresh_token"].as_str() {
            self.credentials.refresh_token = Some(rt.to_string());
        }

        // 从响应中提取用户邮箱
        if let Some(email) = data["account"]["email_address"].as_str() {
            self.credentials.email = Some(email.to_string());
        }

        // 更新过期时间
        let expires_in = data["expires_in"].as_i64().unwrap_or(3600);
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in);
        self.credentials.expire = Some(expires_at.to_rfc3339());
        self.credentials.last_refresh = Some(chrono::Utc::now().to_rfc3339());

        self.save_credentials().await?;

        tracing::info!("[CLAUDE_OAUTH] Token 刷新成功");
        Ok(new_access_token)
    }

    /// 带重试机制的 Token 刷新
    pub async fn refresh_token_with_retry(
        &mut self,
        max_retries: u32,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut last_error = None;

        for attempt in 0..max_retries {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(1 << attempt);
                tracing::info!("[CLAUDE_OAUTH] 第 {} 次重试，等待 {:?}", attempt + 1, delay);
                tokio::time::sleep(delay).await;
            }

            match self.refresh_token().await {
                Ok(token) => return Ok(token),
                Err(e) => {
                    tracing::warn!(
                        "[CLAUDE_OAUTH] Token 刷新第 {} 次尝试失败: {}",
                        attempt + 1,
                        e
                    );
                    last_error = Some(e);
                }
            }
        }

        self.mark_invalid();
        tracing::error!("[CLAUDE_OAUTH] Token 刷新在 {} 次尝试后失败", max_retries);
        Err(last_error.unwrap_or_else(|| create_auth_error("Token 刷新失败，请重新登录")))
    }

    /// 确保 Token 有效，必要时自动刷新
    pub async fn ensure_valid_token(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        if !self.is_token_valid() {
            tracing::info!("[CLAUDE_OAUTH] Token 需要刷新");
            self.refresh_token_with_retry(3).await
        } else {
            self.credentials
                .access_token
                .clone()
                .ok_or_else(|| create_config_error("没有可用的 access_token"))
        }
    }

    /// 标记凭证为无效
    pub fn mark_invalid(&mut self) {
        tracing::warn!("[CLAUDE_OAUTH] 标记凭证为无效");
        self.credentials.access_token = None;
        self.credentials.expire = None;
    }

    /// 获取 OAuth 授权 URL
    pub fn get_auth_url(&self) -> &'static str {
        CLAUDE_AUTH_URL
    }

    /// 获取 OAuth Token URL
    pub fn get_token_url(&self) -> &'static str {
        CLAUDE_TOKEN_URL
    }

    /// 获取 OAuth Client ID
    pub fn get_client_id(&self) -> &'static str {
        CLAUDE_CLIENT_ID
    }

    /// 获取回调 URI
    pub fn get_redirect_uri(&self) -> String {
        format!("http://localhost:{}/callback", self.callback_port)
    }
}
