//! Qwen (通义千问) OAuth Provider
//!
//! 实现 Qwen OAuth 认证流程，与 CLIProxyAPI 对齐。
//! 支持 Token 刷新、重试机制和统一凭证格式。

use super::error::{
    create_auth_error, create_config_error, create_token_refresh_error, ProviderError,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;

// Constants - 与 CLIProxyAPI 对齐
const QWEN_DIR: &str = ".qwen";
const CREDENTIALS_FILE: &str = "oauth_creds.json";
const QWEN_BASE_URL: &str = "https://portal.qwen.ai/v1";

// OAuth 端点和凭证 - 与 CLIProxyAPI 完全一致
const QWEN_TOKEN_URL: &str = "https://chat.qwen.ai/api/v1/oauth2/token";
const QWEN_CLIENT_ID: &str = "f0304373b74a44d2b584a3fb70ca9e56";

pub const QWEN_MODELS: &[&str] = &["qwen3-coder-plus", "qwen3-coder-flash"];

/// Qwen OAuth 凭证存储
///
/// 与 CLIProxyAPI 的 QwenTokenStorage 格式兼容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QwenCredentials {
    /// 访问令牌
    pub access_token: Option<String>,
    /// 刷新令牌
    pub refresh_token: Option<String>,
    /// 令牌类型
    pub token_type: Option<String>,
    /// 资源 URL
    pub resource_url: Option<String>,
    /// 过期时间戳（毫秒）- 兼容旧格式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<i64>,
    /// 过期时间（RFC3339 格式）- 新格式，与 CLIProxyAPI 一致
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire: Option<String>,
    /// 最后刷新时间（RFC3339 格式）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_refresh: Option<String>,
    /// 凭证类型标识
    #[serde(default = "default_qwen_type", rename = "type")]
    pub cred_type: String,
}

fn default_qwen_type() -> String {
    "qwen".to_string()
}

impl Default for QwenCredentials {
    fn default() -> Self {
        Self {
            access_token: None,
            refresh_token: None,
            token_type: Some("Bearer".to_string()),
            resource_url: None,
            expiry_date: None,
            expire: None,
            last_refresh: None,
            cred_type: default_qwen_type(),
        }
    }
}

pub struct QwenProvider {
    pub credentials: QwenCredentials,
    pub client: Client,
}

impl Default for QwenProvider {
    fn default() -> Self {
        Self {
            credentials: QwenCredentials::default(),
            client: Client::new(),
        }
    }
}

impl QwenProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn default_creds_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(QWEN_DIR)
            .join(CREDENTIALS_FILE)
    }

    pub async fn load_credentials(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let path = Self::default_creds_path();

        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            let content = tokio::fs::read_to_string(&path).await?;
            let creds: QwenCredentials = serde_json::from_str(&content)?;
            self.credentials = creds;
        }

        Ok(())
    }

    pub async fn load_credentials_from_path(
        &mut self,
        path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let content = tokio::fs::read_to_string(path).await?;
        let creds: QwenCredentials = serde_json::from_str(&content)?;
        self.credentials = creds;
        Ok(())
    }

    pub async fn save_credentials(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let path = Self::default_creds_path();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let content = serde_json::to_string_pretty(&self.credentials)?;
        tokio::fs::write(&path, content).await?;
        Ok(())
    }

    /// 检查 Token 是否有效
    pub fn is_token_valid(&self) -> bool {
        if self.credentials.access_token.is_none() {
            return false;
        }

        // 优先检查 RFC3339 格式的过期时间
        if let Some(expire_str) = &self.credentials.expire {
            if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(expire_str) {
                let now = chrono::Utc::now();
                // Token 有效期需要超过 30 秒
                return expires > now + chrono::Duration::seconds(30);
            }
        }

        // 兼容旧的毫秒时间戳格式
        if let Some(expiry) = self.credentials.expiry_date {
            let now = chrono::Utc::now().timestamp_millis();
            return expiry > now + 30_000;
        }

        true
    }

    pub fn get_base_url(&self) -> String {
        self.credentials
            .resource_url
            .as_ref()
            .map(|url| {
                let normalized = if url.starts_with("http") {
                    url.clone()
                } else {
                    format!("https://{url}")
                };
                if normalized.ends_with("/v1") {
                    normalized
                } else {
                    format!("{normalized}/v1")
                }
            })
            .unwrap_or_else(|| QWEN_BASE_URL.to_string())
    }

    /// 刷新 Token - 与 CLIProxyAPI 对齐，使用 form-urlencoded 格式
    pub async fn refresh_token(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        let refresh_token = self
            .credentials
            .refresh_token
            .as_ref()
            .ok_or_else(|| create_config_error("没有可用的 refresh_token"))?;

        let client_id =
            std::env::var("QWEN_OAUTH_CLIENT_ID").unwrap_or_else(|_| QWEN_CLIENT_ID.to_string());

        tracing::info!("[QWEN] 正在刷新 Token");

        // 与 CLIProxyAPI 对齐：使用 application/x-www-form-urlencoded 格式
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
            ("client_id", client_id.as_str()),
        ];

        let resp = self
            .client
            .post(QWEN_TOKEN_URL)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(|e| Box::new(ProviderError::from(e)) as Box<dyn Error + Send + Sync>)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!("[QWEN] Token 刷新失败: {} - {}", status, body);
            return Err(create_token_refresh_error(status, &body, "QWEN"));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| Box::new(ProviderError::from(e)) as Box<dyn Error + Send + Sync>)?;

        let new_token = data["access_token"]
            .as_str()
            .ok_or_else(|| create_auth_error("响应中没有 access_token"))?;

        self.credentials.access_token = Some(new_token.to_string());

        if let Some(rt) = data["refresh_token"].as_str() {
            self.credentials.refresh_token = Some(rt.to_string());
        }

        if let Some(resource_url) = data["resource_url"].as_str() {
            self.credentials.resource_url = Some(resource_url.to_string());
        }

        // 更新过期时间（同时保存两种格式以兼容）
        if let Some(expires_in) = data["expires_in"].as_i64() {
            let expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in);
            self.credentials.expire = Some(expires_at.to_rfc3339());
            self.credentials.expiry_date = Some(expires_at.timestamp_millis());
        }

        // 更新最后刷新时间
        self.credentials.last_refresh = Some(chrono::Utc::now().to_rfc3339());

        // 保存刷新后的凭证
        self.save_credentials().await?;

        tracing::info!("[QWEN] Token 刷新成功");
        Ok(new_token.to_string())
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
                tracing::info!("[QWEN] 第 {} 次重试，等待 {:?}", attempt + 1, delay);
                tokio::time::sleep(delay).await;
            }

            match self.refresh_token().await {
                Ok(token) => return Ok(token),
                Err(e) => {
                    tracing::warn!("[QWEN] Token 刷新第 {} 次尝试失败: {}", attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        tracing::error!("[QWEN] Token 刷新在 {} 次尝试后失败", max_retries);
        Err(last_error.unwrap_or_else(|| create_auth_error("Token 刷新失败，请重新登录")))
    }

    /// 确保 Token 有效，必要时自动刷新
    pub async fn ensure_valid_token(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        if !self.is_token_valid() {
            tracing::info!("[QWEN] Token 需要刷新");
            self.refresh_token_with_retry(3).await
        } else {
            self.credentials
                .access_token
                .clone()
                .ok_or_else(|| create_config_error("没有可用的 access_token"))
        }
    }

    pub async fn chat_completions(
        &self,
        request: &serde_json::Value,
    ) -> Result<reqwest::Response, Box<dyn Error + Send + Sync>> {
        let token = self
            .credentials
            .access_token
            .as_ref()
            .ok_or_else(|| create_config_error("没有可用的 access_token"))?;

        let base_url = self.get_base_url();
        let url = format!("{base_url}/chat/completions");

        // Ensure model is valid
        let mut req_body = request.clone();
        if let Some(model) = req_body.get("model").and_then(|m| m.as_str()) {
            if !QWEN_MODELS.contains(&model) {
                req_body["model"] = serde_json::json!(QWEN_MODELS[0]);
            }
        }

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .header("X-DashScope-AuthType", "qwen-oauth")
            .json(&req_body)
            .send()
            .await?;

        Ok(resp)
    }
}
