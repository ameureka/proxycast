//! OpenAI Custom Provider (自定义 OpenAI 兼容 API)
use crate::models::openai::ChatCompletionRequest;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAICustomConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub enabled: bool,
}

pub struct OpenAICustomProvider {
    pub config: OpenAICustomConfig,
    pub client: Client,
}

impl Default for OpenAICustomProvider {
    fn default() -> Self {
        Self {
            config: OpenAICustomConfig::default(),
            client: Client::new(),
        }
    }
}

impl OpenAICustomProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// 使用 API key 和 base_url 创建 Provider
    pub fn with_config(api_key: String, base_url: Option<String>) -> Self {
        Self {
            config: OpenAICustomConfig {
                api_key: Some(api_key),
                base_url,
                enabled: true,
            },
            client: Client::new(),
        }
    }

    pub fn get_base_url(&self) -> String {
        self.config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com".to_string())
    }

    pub fn is_configured(&self) -> bool {
        self.config.api_key.is_some() && self.config.enabled
    }

    /// 构建完整的 API URL
    /// 智能处理用户输入的 base_url，无论是否带 /v1 都能正确工作
    fn build_url(&self, endpoint: &str) -> String {
        let base = self.get_base_url();
        let base = base.trim_end_matches('/');

        // 如果用户输入了带 /v1 的 URL，直接拼接 endpoint
        // 否则拼接 /v1/endpoint
        if base.ends_with("/v1") {
            format!("{}/{}", base, endpoint)
        } else {
            format!("{}/v1/{}", base, endpoint)
        }
    }

    /// 调用 OpenAI API（使用类型化请求）
    pub async fn call_api(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<reqwest::Response, Box<dyn Error + Send + Sync>> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or("OpenAI API key not configured")?;

        let url = self.build_url("chat/completions");

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await?;

        Ok(resp)
    }

    pub async fn chat_completions(
        &self,
        request: &serde_json::Value,
    ) -> Result<reqwest::Response, Box<dyn Error + Send + Sync>> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or("OpenAI API key not configured")?;

        let url = self.build_url("chat/completions");

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await?;

        Ok(resp)
    }

    pub async fn list_models(&self) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or("OpenAI API key not configured")?;

        let url = self.build_url("models");

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Failed to list models: {status} - {body}").into());
        }

        let data: serde_json::Value = resp.json().await?;
        Ok(data)
    }
}
