//! Gemini CLI OAuth Provider
//!
//! 实现 Google Gemini OAuth 认证流程，与 CLIProxyAPI 对齐。
//! 支持 Token 刷新、重试机制和统一凭证格式。

use super::error::{
    create_auth_error, create_config_error, create_token_refresh_error, ProviderError,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;

// Constants - 与 CLIProxyAPI 对齐
const CODE_ASSIST_ENDPOINT: &str = "https://cloudcode-pa.googleapis.com";
const CODE_ASSIST_API_VERSION: &str = "v1internal";
const CREDENTIALS_DIR: &str = ".gemini";
const CREDENTIALS_FILE: &str = "oauth_creds.json";

// OAuth 端点
const GEMINI_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

// OAuth 凭证从环境变量读取
fn get_oauth_client_id() -> String {
    std::env::var("GEMINI_OAUTH_CLIENT_ID").unwrap_or_default()
}

fn get_oauth_client_secret() -> String {
    std::env::var("GEMINI_OAUTH_CLIENT_SECRET").unwrap_or_default()
}

#[allow(dead_code)]
pub const GEMINI_MODELS: &[&str] = &[
    "gemini-2.5-flash",
    "gemini-2.5-flash-lite",
    "gemini-2.5-pro",
    "gemini-2.5-pro-preview-06-05",
    "gemini-2.5-flash-preview-09-2025",
    "gemini-3-pro-preview",
];

/// Gemini OAuth 凭证存储
///
/// 与 CLIProxyAPI 的 GeminiTokenStorage 格式兼容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiCredentials {
    /// 访问令牌
    pub access_token: Option<String>,
    /// 刷新令牌
    pub refresh_token: Option<String>,
    /// 令牌类型
    pub token_type: Option<String>,
    /// 过期时间戳（毫秒）- 兼容旧格式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<i64>,
    /// 过期时间（RFC3339 格式）- 新格式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire: Option<String>,
    /// OAuth 作用域
    pub scope: Option<String>,
    /// 用户邮箱
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// 最后刷新时间（RFC3339 格式）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_refresh: Option<String>,
    /// 凭证类型标识
    #[serde(default = "default_gemini_type", rename = "type")]
    pub cred_type: String,
    /// 嵌套的 token 对象（兼容 CLIProxyAPI 格式）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<GeminiTokenInfo>,
}

fn default_gemini_type() -> String {
    "gemini".to_string()
}

/// 嵌套的 Token 信息（兼容 CLIProxyAPI 格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiTokenInfo {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_uri: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub scopes: Option<Vec<String>>,
}

impl Default for GeminiCredentials {
    fn default() -> Self {
        Self {
            access_token: None,
            refresh_token: None,
            token_type: Some("Bearer".to_string()),
            expiry_date: None,
            expire: None,
            scope: None,
            email: None,
            last_refresh: None,
            cred_type: default_gemini_type(),
            token: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiContent {
    pub role: String,
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiRequest {
    pub model: String,
    pub project: String,
    pub request: GeminiRequestBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiRequestBody {
    pub contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiResponse {
    pub candidates: Option<Vec<GeminiCandidate>>,
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiCandidate {
    pub content: Option<GeminiContent>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiUsageMetadata {
    pub prompt_token_count: Option<i32>,
    pub candidates_token_count: Option<i32>,
    pub total_token_count: Option<i32>,
}

pub struct GeminiProvider {
    pub credentials: GeminiCredentials,
    pub project_id: Option<String>,
    pub client: Client,
}

impl Default for GeminiProvider {
    fn default() -> Self {
        Self {
            credentials: GeminiCredentials::default(),
            project_id: None,
            client: Client::new(),
        }
    }
}

impl GeminiProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn default_creds_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(CREDENTIALS_DIR)
            .join(CREDENTIALS_FILE)
    }

    pub async fn load_credentials(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let path = Self::default_creds_path();

        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            let content = tokio::fs::read_to_string(&path).await?;
            let creds: GeminiCredentials = serde_json::from_str(&content)?;
            self.credentials = creds;
        }

        Ok(())
    }

    pub async fn load_credentials_from_path(
        &mut self,
        path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let content = tokio::fs::read_to_string(path).await?;
        let creds: GeminiCredentials = serde_json::from_str(&content)?;
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
                // Token 有效期需要超过 5 分钟
                return expires > now + chrono::Duration::minutes(5);
            }
        }

        // 兼容旧的毫秒时间戳格式
        if let Some(expiry) = self.credentials.expiry_date {
            let now = chrono::Utc::now().timestamp_millis();
            return expiry > now + 300_000;
        }

        true
    }

    /// 刷新 Token
    pub async fn refresh_token(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        let refresh_token = self
            .credentials
            .refresh_token
            .as_ref()
            .ok_or_else(|| create_config_error("没有可用的 refresh_token"))?;

        let client_id = get_oauth_client_id();
        let client_secret = get_oauth_client_secret();

        tracing::info!("[GEMINI] 正在刷新 Token");

        let params = [
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ];

        let resp = self
            .client
            .post(GEMINI_TOKEN_URL)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(|e| Box::new(ProviderError::from(e)) as Box<dyn Error + Send + Sync>)?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!("[GEMINI] Token 刷新失败: {} - {}", status, body);
            return Err(create_token_refresh_error(status, &body, "GEMINI"));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| Box::new(ProviderError::from(e)) as Box<dyn Error + Send + Sync>)?;

        let new_token = data["access_token"]
            .as_str()
            .ok_or_else(|| create_auth_error("响应中没有 access_token"))?;

        self.credentials.access_token = Some(new_token.to_string());

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

        tracing::info!("[GEMINI] Token 刷新成功");
        Ok(new_token.to_string())
    }

    /// 带重试机制的 Token 刷新
    ///
    /// 最多重试 `max_retries` 次，使用指数退避策略
    pub async fn refresh_token_with_retry(
        &mut self,
        max_retries: u32,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut last_error = None;

        for attempt in 0..max_retries {
            if attempt > 0 {
                // 指数退避: 1s, 2s, 4s, ...
                let delay = std::time::Duration::from_secs(1 << attempt);
                tracing::info!("[GEMINI] 第 {} 次重试，等待 {:?}", attempt + 1, delay);
                tokio::time::sleep(delay).await;
            }

            match self.refresh_token().await {
                Ok(token) => return Ok(token),
                Err(e) => {
                    tracing::warn!("[GEMINI] Token 刷新第 {} 次尝试失败: {}", attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        tracing::error!("[GEMINI] Token 刷新在 {} 次尝试后失败", max_retries);
        Err(last_error.unwrap_or_else(|| create_auth_error("Token 刷新失败，请重新登录")))
    }

    /// 确保 Token 有效，必要时自动刷新
    pub async fn ensure_valid_token(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        if !self.is_token_valid() {
            tracing::info!("[GEMINI] Token 需要刷新");
            self.refresh_token_with_retry(3).await
        } else {
            self.credentials
                .access_token
                .clone()
                .ok_or_else(|| "没有可用的 access_token".into())
        }
    }

    pub fn get_api_url(&self, action: &str) -> String {
        format!("{CODE_ASSIST_ENDPOINT}/{CODE_ASSIST_API_VERSION}:{action}")
    }

    pub async fn call_api(
        &self,
        action: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        let token = self
            .credentials
            .access_token
            .as_ref()
            .ok_or("No access token")?;

        let url = self.get_api_url(action);

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("API call failed: {status} - {body}").into());
        }

        let data: serde_json::Value = resp.json().await?;
        Ok(data)
    }

    pub async fn discover_project(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        if let Some(ref project_id) = self.project_id {
            return Ok(project_id.clone());
        }

        let body = serde_json::json!({
            "cloudaicompanionProject": "",
            "metadata": {
                "ideType": "IDE_UNSPECIFIED",
                "platform": "PLATFORM_UNSPECIFIED",
                "pluginType": "GEMINI",
                "duetProject": ""
            }
        });

        let resp = self.call_api("loadCodeAssist", &body).await?;

        if let Some(project) = resp["cloudaicompanionProject"].as_str() {
            if !project.is_empty() {
                self.project_id = Some(project.to_string());
                return Ok(project.to_string());
            }
        }

        // Need to onboard
        let onboard_body = serde_json::json!({
            "tierId": "free-tier",
            "cloudaicompanionProject": "",
            "metadata": {
                "ideType": "IDE_UNSPECIFIED",
                "platform": "PLATFORM_UNSPECIFIED",
                "pluginType": "GEMINI",
                "duetProject": ""
            }
        });

        let mut lro_resp = self.call_api("onboardUser", &onboard_body).await?;

        // Poll until done
        for _ in 0..30 {
            if lro_resp["done"].as_bool().unwrap_or(false) {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            lro_resp = self.call_api("onboardUser", &onboard_body).await?;
        }

        let project_id = lro_resp["response"]["cloudaicompanionProject"]["id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        if project_id.is_empty() {
            return Err("Failed to discover project ID".into());
        }

        self.project_id = Some(project_id.clone());
        Ok(project_id)
    }
}

// ============ Gemini API Key Provider ============

/// Default Gemini API base URL
pub const GEMINI_API_BASE_URL: &str = "https://generativelanguage.googleapis.com";

/// Gemini API Key Provider for multi-account load balancing
///
/// This provider supports:
/// - Multiple API keys with round-robin load balancing
/// - Per-key custom base URLs
/// - Model exclusion filtering (to be implemented in task 11.2)
#[derive(Debug, Clone)]
pub struct GeminiApiKeyCredential {
    /// Credential ID
    pub id: String,
    /// API Key
    pub api_key: String,
    /// Custom base URL (optional)
    pub base_url: Option<String>,
    /// Excluded models (supports wildcards)
    pub excluded_models: Vec<String>,
    /// Per-key proxy URL (optional)
    pub proxy_url: Option<String>,
    /// Whether this credential is disabled
    pub disabled: bool,
}

impl GeminiApiKeyCredential {
    /// Create a new Gemini API Key credential
    pub fn new(id: String, api_key: String) -> Self {
        Self {
            id,
            api_key,
            base_url: None,
            excluded_models: Vec::new(),
            proxy_url: None,
            disabled: false,
        }
    }

    /// Set custom base URL
    pub fn with_base_url(mut self, base_url: Option<String>) -> Self {
        self.base_url = base_url;
        self
    }

    /// Set excluded models
    pub fn with_excluded_models(mut self, excluded_models: Vec<String>) -> Self {
        self.excluded_models = excluded_models;
        self
    }

    /// Set proxy URL
    pub fn with_proxy_url(mut self, proxy_url: Option<String>) -> Self {
        self.proxy_url = proxy_url;
        self
    }

    /// Set disabled state
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Get the effective base URL (custom or default)
    pub fn get_base_url(&self) -> &str {
        self.base_url.as_deref().unwrap_or(GEMINI_API_BASE_URL)
    }

    /// Check if this credential is available (not disabled)
    pub fn is_available(&self) -> bool {
        !self.disabled
    }

    /// Check if this credential supports the given model
    /// Returns false if the model matches any exclusion pattern
    pub fn supports_model(&self, model: &str) -> bool {
        !self.excluded_models.iter().any(|pattern| {
            if pattern.contains('*') {
                // Simple wildcard matching
                let pattern = pattern.replace('*', ".*");
                regex::Regex::new(&format!("^{}$", pattern))
                    .map(|re| re.is_match(model))
                    .unwrap_or(false)
            } else {
                pattern == model
            }
        })
    }

    /// Build the API URL for a given model and action
    pub fn build_api_url(&self, model: &str, action: &str) -> String {
        format!("{}/v1beta/models/{}:{}", self.get_base_url(), model, action)
    }
}

/// Gemini API Key Provider
///
/// Manages multiple Gemini API keys with load balancing support.
/// Integrates with the credential pool system for round-robin selection.
pub struct GeminiApiKeyProvider {
    /// HTTP client
    pub client: Client,
}

impl Default for GeminiApiKeyProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GeminiApiKeyProvider {
    /// Create a new Gemini API Key provider
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Create a provider with a custom HTTP client
    pub fn with_client(client: Client) -> Self {
        Self { client }
    }

    /// Make a generateContent request using the given credential
    pub async fn generate_content(
        &self,
        credential: &GeminiApiKeyCredential,
        model: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        let url = credential.build_api_url(model, "generateContent");

        let resp = self
            .client
            .post(&url)
            .header("x-goog-api-key", &credential.api_key)
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Gemini API call failed: {status} - {body}").into());
        }

        let data: serde_json::Value = resp.json().await?;
        Ok(data)
    }

    /// Make a streamGenerateContent request using the given credential
    pub async fn stream_generate_content(
        &self,
        credential: &GeminiApiKeyCredential,
        model: &str,
        body: &serde_json::Value,
    ) -> Result<reqwest::Response, Box<dyn Error + Send + Sync>> {
        let url = format!(
            "{}?alt=sse",
            credential.build_api_url(model, "streamGenerateContent")
        );

        let resp = self
            .client
            .post(&url)
            .header("x-goog-api-key", &credential.api_key)
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Gemini API stream call failed: {status} - {body}").into());
        }

        Ok(resp)
    }

    /// List available models using the given credential
    pub async fn list_models(
        &self,
        credential: &GeminiApiKeyCredential,
    ) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/v1beta/models", credential.get_base_url());

        let resp = self
            .client
            .get(&url)
            .header("x-goog-api-key", &credential.api_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Gemini API list models failed: {status} - {body}").into());
        }

        let data: serde_json::Value = resp.json().await?;
        Ok(data)
    }
}

#[cfg(test)]
mod gemini_api_key_tests {
    use super::*;

    #[test]
    fn test_gemini_api_key_credential_new() {
        let cred = GeminiApiKeyCredential::new("test-id".to_string(), "test-key".to_string());
        assert_eq!(cred.id, "test-id");
        assert_eq!(cred.api_key, "test-key");
        assert!(cred.base_url.is_none());
        assert!(cred.excluded_models.is_empty());
        assert!(cred.proxy_url.is_none());
        assert!(!cred.disabled);
    }

    #[test]
    fn test_gemini_api_key_credential_with_base_url() {
        let cred = GeminiApiKeyCredential::new("test-id".to_string(), "test-key".to_string())
            .with_base_url(Some("https://custom.api.com".to_string()));
        assert_eq!(cred.get_base_url(), "https://custom.api.com");
    }

    #[test]
    fn test_gemini_api_key_credential_default_base_url() {
        let cred = GeminiApiKeyCredential::new("test-id".to_string(), "test-key".to_string());
        assert_eq!(cred.get_base_url(), GEMINI_API_BASE_URL);
    }

    #[test]
    fn test_gemini_api_key_credential_is_available() {
        let cred = GeminiApiKeyCredential::new("test-id".to_string(), "test-key".to_string());
        assert!(cred.is_available());

        let disabled_cred = cred.with_disabled(true);
        assert!(!disabled_cred.is_available());
    }

    #[test]
    fn test_gemini_api_key_credential_supports_model() {
        let cred = GeminiApiKeyCredential::new("test-id".to_string(), "test-key".to_string())
            .with_excluded_models(vec![
                "gemini-2.5-pro".to_string(),
                "gemini-*-preview".to_string(),
            ]);

        // Exact match exclusion
        assert!(!cred.supports_model("gemini-2.5-pro"));

        // Wildcard exclusion
        assert!(!cred.supports_model("gemini-3-preview"));
        assert!(!cred.supports_model("gemini-2.5-preview"));

        // Not excluded
        assert!(cred.supports_model("gemini-2.5-flash"));
        assert!(cred.supports_model("gemini-2.0-flash"));
    }

    #[test]
    fn test_gemini_api_key_credential_build_api_url() {
        let cred = GeminiApiKeyCredential::new("test-id".to_string(), "test-key".to_string());
        let url = cred.build_api_url("gemini-2.5-flash", "generateContent");
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent"
        );

        let custom_cred = cred.with_base_url(Some("https://custom.api.com".to_string()));
        let custom_url = custom_cred.build_api_url("gemini-2.5-flash", "generateContent");
        assert_eq!(
            custom_url,
            "https://custom.api.com/v1beta/models/gemini-2.5-flash:generateContent"
        );
    }

    #[test]
    fn test_gemini_api_key_provider_new() {
        let provider = GeminiApiKeyProvider::new();
        // Just verify it can be created
        assert!(true);
        let _ = provider;
    }
}
