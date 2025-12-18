//! Amp CLI 路由器
//!
//! 处理 Amp CLI 的请求路由，支持 `/api/provider/{provider}/v1/*` 模式。
//!
//! # 功能
//!
//! - 解析 Amp CLI 请求路径
//! - 应用模型映射（将不可用模型映射到可用替代）
//! - 识别管理路由（/api/auth/*, /api/user/*）
//!
//! # 示例
//!
//! ```rust
//! use proxycast::router::AmpRouter;
//! use proxycast::config::AmpConfig;
//!
//! let config = AmpConfig::default();
//! let router = AmpRouter::new(config);
//!
//! // 解析 provider 路由
//! let result = router.parse_provider_route("/api/provider/anthropic/v1/messages");
//! assert!(result.is_some());
//! ```

use crate::config::{AmpConfig, AmpModelMapping};

/// Amp 路由解析结果
#[derive(Debug, Clone, PartialEq)]
pub struct AmpRouteMatch {
    /// Provider 名称（如 "anthropic", "openai"）
    pub provider: String,
    /// API 版本（如 "v1"）
    pub version: String,
    /// 端点路径（如 "messages", "chat/completions"）
    pub endpoint: String,
    /// 完整的剩余路径
    pub remaining_path: String,
}

impl AmpRouteMatch {
    /// 是否是 Claude/Anthropic 协议
    pub fn is_anthropic_protocol(&self) -> bool {
        self.provider == "anthropic" || self.endpoint == "messages"
    }

    /// 是否是 OpenAI 协议
    pub fn is_openai_protocol(&self) -> bool {
        self.provider == "openai" || self.endpoint.contains("chat/completions")
    }

    /// 获取目标 URL 路径（不含 /api/provider/{provider} 前缀）
    pub fn target_path(&self) -> String {
        format!("/{}/{}", self.version, self.remaining_path)
    }
}

/// Amp CLI 路由器
///
/// 处理 Amp CLI 的请求路由和模型映射。
#[derive(Debug, Clone)]
pub struct AmpRouter {
    /// 上游 URL
    upstream_url: Option<String>,
    /// 模型映射（from -> to）
    model_mappings: Vec<AmpModelMapping>,
    /// 是否限制管理端点只能从 localhost 访问
    restrict_management_to_localhost: bool,
}

impl AmpRouter {
    /// 创建新的 Amp 路由器
    pub fn new(config: AmpConfig) -> Self {
        Self {
            upstream_url: config.upstream_url,
            model_mappings: config.model_mappings,
            restrict_management_to_localhost: config.restrict_management_to_localhost,
        }
    }

    /// 从配置组件创建路由器
    pub fn from_parts(
        upstream_url: Option<String>,
        model_mappings: Vec<AmpModelMapping>,
        restrict_management_to_localhost: bool,
    ) -> Self {
        Self {
            upstream_url,
            model_mappings,
            restrict_management_to_localhost,
        }
    }

    /// 获取上游 URL
    pub fn upstream_url(&self) -> Option<&str> {
        self.upstream_url.as_deref()
    }

    /// 是否限制管理端点到 localhost
    pub fn restrict_management_to_localhost(&self) -> bool {
        self.restrict_management_to_localhost
    }

    /// 解析 provider 路由
    ///
    /// 支持的路径格式：
    /// - `/api/provider/{provider}/v1/messages`
    /// - `/api/provider/{provider}/v1/chat/completions`
    /// - `/api/provider/{provider}/v1/*`
    ///
    /// # 返回
    ///
    /// 如果路径匹配 `/api/provider/{provider}/v1/*` 模式，返回 `Some(AmpRouteMatch)`；
    /// 否则返回 `None`。
    pub fn parse_provider_route(&self, path: &str) -> Option<AmpRouteMatch> {
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();

        // 检查是否匹配 api/provider/{provider}/v1/* 模式
        // 最少需要 5 个部分: api, provider, {provider_name}, v1, {endpoint}
        if parts.len() < 5 {
            return None;
        }

        if parts[0] != "api" || parts[1] != "provider" {
            return None;
        }

        let provider = parts[2].to_string();
        let version = parts[3].to_string();

        // 验证版本格式（应该是 v1, v2 等）
        if !version.starts_with('v') {
            return None;
        }

        // 剩余路径（从 endpoint 开始）
        let remaining_path = parts[4..].join("/");
        let endpoint = parts[4].to_string();

        Some(AmpRouteMatch {
            provider,
            version,
            endpoint,
            remaining_path,
        })
    }

    /// 应用模型映射
    ///
    /// 如果模型在映射表中，返回映射后的模型名；否则返回原模型名。
    ///
    /// # 示例
    ///
    /// ```rust
    /// // 配置: claude-opus-4.5 -> claude-sonnet-4
    /// let mapped = router.apply_model_mapping("claude-opus-4.5");
    /// assert_eq!(mapped, "claude-sonnet-4");
    /// ```
    pub fn apply_model_mapping(&self, model: &str) -> String {
        for mapping in &self.model_mappings {
            if mapping.from == model {
                return mapping.to.clone();
            }
        }
        model.to_string()
    }

    /// 转换请求体中的模型名称
    ///
    /// 在 JSON 请求体中查找 "model" 字段并应用模型映射。
    /// 支持 OpenAI 和 Anthropic 格式的请求。
    ///
    /// # 返回
    ///
    /// 返回一个元组 `(transformed_body, original_model, mapped_model)`：
    /// - `transformed_body`: 转换后的 JSON 请求体
    /// - `original_model`: 原始模型名（如果存在）
    /// - `mapped_model`: 映射后的模型名（如果发生了映射）
    ///
    /// # 示例
    ///
    /// ```rust
    /// let body = r#"{"model": "claude-opus-4.5", "messages": []}"#;
    /// let (transformed, original, mapped) = router.transform_request_model(body);
    /// // 如果配置了 claude-opus-4.5 -> claude-sonnet-4
    /// // transformed 将包含 "model": "claude-sonnet-4"
    /// ```
    pub fn transform_request_model(&self, body: &str) -> (String, Option<String>, Option<String>) {
        // 尝试解析 JSON
        let mut json: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return (body.to_string(), None, None),
        };

        // 查找并转换 model 字段
        let original_model = json
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mapped_model = if let Some(ref model) = original_model {
            let mapped = self.apply_model_mapping(model);
            if mapped != *model {
                // 更新 JSON 中的 model 字段
                if let Some(obj) = json.as_object_mut() {
                    obj.insert(
                        "model".to_string(),
                        serde_json::Value::String(mapped.clone()),
                    );
                }
                Some(mapped)
            } else {
                None
            }
        } else {
            None
        };

        // 序列化回 JSON 字符串
        let transformed = serde_json::to_string(&json).unwrap_or_else(|_| body.to_string());

        (transformed, original_model, mapped_model)
    }

    /// 转换 JSON Value 中的模型名称
    ///
    /// 直接操作 serde_json::Value，避免额外的序列化/反序列化开销。
    ///
    /// # 返回
    ///
    /// 返回一个元组 `(original_model, mapped_model)`：
    /// - `original_model`: 原始模型名（如果存在）
    /// - `mapped_model`: 映射后的模型名（如果发生了映射）
    pub fn transform_request_model_value(
        &self,
        json: &mut serde_json::Value,
    ) -> (Option<String>, Option<String>) {
        // 查找原始 model 字段
        let original_model = json
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mapped_model = if let Some(ref model) = original_model {
            let mapped = self.apply_model_mapping(model);
            if mapped != *model {
                // 更新 JSON 中的 model 字段
                if let Some(obj) = json.as_object_mut() {
                    obj.insert(
                        "model".to_string(),
                        serde_json::Value::String(mapped.clone()),
                    );
                }
                Some(mapped)
            } else {
                None
            }
        } else {
            None
        };

        (original_model, mapped_model)
    }

    /// 批量应用模型映射
    ///
    /// 对多个模型名称应用映射，返回映射结果列表。
    /// 用于处理包含多个模型引用的请求。
    pub fn apply_model_mappings_batch(&self, models: &[&str]) -> Vec<String> {
        models
            .iter()
            .map(|model| self.apply_model_mapping(model))
            .collect()
    }

    /// 获取模型映射的反向查找
    ///
    /// 给定一个目标模型名，返回所有映射到该模型的源模型名。
    /// 用于调试和日志记录。
    pub fn get_reverse_mappings(&self, target_model: &str) -> Vec<String> {
        self.model_mappings
            .iter()
            .filter(|m| m.to == target_model)
            .map(|m| m.from.clone())
            .collect()
    }

    /// 检查是否有模型映射
    pub fn has_model_mapping(&self, model: &str) -> bool {
        self.model_mappings.iter().any(|m| m.from == model)
    }

    /// 获取所有模型映射
    pub fn model_mappings(&self) -> &[AmpModelMapping] {
        &self.model_mappings
    }

    /// 添加模型映射
    pub fn add_model_mapping(&mut self, from: &str, to: &str) {
        self.model_mappings.push(AmpModelMapping {
            from: from.to_string(),
            to: to.to_string(),
        });
    }

    /// 移除模型映射
    pub fn remove_model_mapping(&mut self, from: &str) -> bool {
        let len_before = self.model_mappings.len();
        self.model_mappings.retain(|m| m.from != from);
        self.model_mappings.len() < len_before
    }

    /// 检查是否是管理路由
    ///
    /// 管理路由包括：
    /// - `/api/auth/*` - 认证相关
    /// - `/api/user/*` - 用户相关
    pub fn is_management_route(&self, path: &str) -> bool {
        let path = path.trim_start_matches('/');
        path.starts_with("api/auth/") || path.starts_with("api/user/")
    }

    /// 检查是否是 Amp 路由（provider 路由或管理路由）
    pub fn is_amp_route(&self, path: &str) -> bool {
        self.parse_provider_route(path).is_some() || self.is_management_route(path)
    }

    /// 获取管理路由的上游路径
    ///
    /// 将本地管理路由转换为上游 URL 路径。
    pub fn get_management_upstream_path(&self, path: &str) -> Option<String> {
        if !self.is_management_route(path) {
            return None;
        }

        let upstream = self.upstream_url.as_ref()?;
        let path = path.trim_start_matches('/');
        Some(format!("{}/{}", upstream.trim_end_matches('/'), path))
    }
}

impl Default for AmpRouter {
    fn default() -> Self {
        Self::new(AmpConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_router() -> AmpRouter {
        let config = AmpConfig {
            upstream_url: Some("https://ampcode.com".to_string()),
            model_mappings: vec![
                AmpModelMapping {
                    from: "claude-opus-4.5".to_string(),
                    to: "claude-sonnet-4".to_string(),
                },
                AmpModelMapping {
                    from: "gpt-5".to_string(),
                    to: "gemini-2.5-pro".to_string(),
                },
            ],
            restrict_management_to_localhost: false,
        };
        AmpRouter::new(config)
    }

    #[test]
    fn test_parse_provider_route_messages() {
        let router = create_test_router();

        let result = router
            .parse_provider_route("/api/provider/anthropic/v1/messages")
            .unwrap();

        assert_eq!(result.provider, "anthropic");
        assert_eq!(result.version, "v1");
        assert_eq!(result.endpoint, "messages");
        assert_eq!(result.remaining_path, "messages");
        assert!(result.is_anthropic_protocol());
    }

    #[test]
    fn test_parse_provider_route_chat_completions() {
        let router = create_test_router();

        let result = router
            .parse_provider_route("/api/provider/openai/v1/chat/completions")
            .unwrap();

        assert_eq!(result.provider, "openai");
        assert_eq!(result.version, "v1");
        assert_eq!(result.endpoint, "chat");
        assert_eq!(result.remaining_path, "chat/completions");
        assert!(result.is_openai_protocol());
    }

    #[test]
    fn test_parse_provider_route_without_leading_slash() {
        let router = create_test_router();

        let result = router
            .parse_provider_route("api/provider/anthropic/v1/messages")
            .unwrap();

        assert_eq!(result.provider, "anthropic");
        assert_eq!(result.version, "v1");
    }

    #[test]
    fn test_parse_provider_route_invalid_paths() {
        let router = create_test_router();

        // 路径太短
        assert!(router.parse_provider_route("/api/provider").is_none());
        assert!(router
            .parse_provider_route("/api/provider/anthropic")
            .is_none());
        assert!(router
            .parse_provider_route("/api/provider/anthropic/v1")
            .is_none());

        // 不是 api/provider 开头
        assert!(router.parse_provider_route("/v1/messages").is_none());
        assert!(router
            .parse_provider_route("/other/provider/anthropic/v1/messages")
            .is_none());

        // 版本格式不对
        assert!(router
            .parse_provider_route("/api/provider/anthropic/1/messages")
            .is_none());
    }

    #[test]
    fn test_apply_model_mapping() {
        let router = create_test_router();

        // 有映射的模型
        assert_eq!(
            router.apply_model_mapping("claude-opus-4.5"),
            "claude-sonnet-4"
        );
        assert_eq!(router.apply_model_mapping("gpt-5"), "gemini-2.5-pro");

        // 没有映射的模型，返回原值
        assert_eq!(
            router.apply_model_mapping("claude-sonnet-4"),
            "claude-sonnet-4"
        );
        assert_eq!(router.apply_model_mapping("unknown-model"), "unknown-model");
    }

    #[test]
    fn test_has_model_mapping() {
        let router = create_test_router();

        assert!(router.has_model_mapping("claude-opus-4.5"));
        assert!(router.has_model_mapping("gpt-5"));
        assert!(!router.has_model_mapping("claude-sonnet-4"));
    }

    #[test]
    fn test_is_management_route() {
        let router = create_test_router();

        // 管理路由
        assert!(router.is_management_route("/api/auth/login"));
        assert!(router.is_management_route("/api/auth/callback"));
        assert!(router.is_management_route("/api/user/profile"));
        assert!(router.is_management_route("api/auth/token")); // 无前导斜杠

        // 非管理路由
        assert!(!router.is_management_route("/api/provider/anthropic/v1/messages"));
        assert!(!router.is_management_route("/v1/messages"));
        assert!(!router.is_management_route("/api/other/path"));
    }

    #[test]
    fn test_is_amp_route() {
        let router = create_test_router();

        // Amp 路由
        assert!(router.is_amp_route("/api/provider/anthropic/v1/messages"));
        assert!(router.is_amp_route("/api/auth/login"));
        assert!(router.is_amp_route("/api/user/profile"));

        // 非 Amp 路由
        assert!(!router.is_amp_route("/v1/messages"));
        assert!(!router.is_amp_route("/health"));
    }

    #[test]
    fn test_get_management_upstream_path() {
        let router = create_test_router();

        let path = router
            .get_management_upstream_path("/api/auth/login")
            .unwrap();
        assert_eq!(path, "https://ampcode.com/api/auth/login");

        let path = router
            .get_management_upstream_path("/api/user/profile")
            .unwrap();
        assert_eq!(path, "https://ampcode.com/api/user/profile");

        // 非管理路由返回 None
        assert!(router
            .get_management_upstream_path("/api/provider/anthropic/v1/messages")
            .is_none());
    }

    #[test]
    fn test_get_management_upstream_path_no_upstream() {
        let router = AmpRouter::new(AmpConfig::default());

        // 没有配置上游 URL 时返回 None
        assert!(router
            .get_management_upstream_path("/api/auth/login")
            .is_none());
    }

    #[test]
    fn test_target_path() {
        let router = create_test_router();

        let result = router
            .parse_provider_route("/api/provider/anthropic/v1/messages")
            .unwrap();
        assert_eq!(result.target_path(), "/v1/messages");

        let result = router
            .parse_provider_route("/api/provider/openai/v1/chat/completions")
            .unwrap();
        assert_eq!(result.target_path(), "/v1/chat/completions");
    }

    #[test]
    fn test_add_and_remove_model_mapping() {
        let mut router = AmpRouter::default();

        // 添加映射
        router.add_model_mapping("model-a", "model-b");
        assert!(router.has_model_mapping("model-a"));
        assert_eq!(router.apply_model_mapping("model-a"), "model-b");

        // 移除映射
        assert!(router.remove_model_mapping("model-a"));
        assert!(!router.has_model_mapping("model-a"));
        assert_eq!(router.apply_model_mapping("model-a"), "model-a");

        // 移除不存在的映射
        assert!(!router.remove_model_mapping("nonexistent"));
    }

    #[test]
    fn test_transform_request_model() {
        let router = create_test_router();

        // 测试 OpenAI 格式请求
        let body =
            r#"{"model": "claude-opus-4.5", "messages": [{"role": "user", "content": "Hello"}]}"#;
        let (transformed, original, mapped) = router.transform_request_model(body);

        assert_eq!(original, Some("claude-opus-4.5".to_string()));
        assert_eq!(mapped, Some("claude-sonnet-4".to_string()));
        assert!(transformed.contains("claude-sonnet-4"));
        assert!(!transformed.contains("claude-opus-4.5"));

        // 测试无映射的模型
        let body2 = r#"{"model": "gpt-4", "messages": []}"#;
        let (transformed2, original2, mapped2) = router.transform_request_model(body2);

        assert_eq!(original2, Some("gpt-4".to_string()));
        assert_eq!(mapped2, None);
        assert!(transformed2.contains("gpt-4"));
    }

    #[test]
    fn test_transform_request_model_value() {
        let router = create_test_router();

        // 测试直接操作 JSON Value
        let mut json: serde_json::Value = serde_json::json!({
            "model": "gpt-5",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let (original, mapped) = router.transform_request_model_value(&mut json);

        assert_eq!(original, Some("gpt-5".to_string()));
        assert_eq!(mapped, Some("gemini-2.5-pro".to_string()));
        assert_eq!(json["model"], "gemini-2.5-pro");
    }

    #[test]
    fn test_transform_request_model_no_model_field() {
        let router = create_test_router();

        // 测试没有 model 字段的请求
        let body = r#"{"messages": [{"role": "user", "content": "Hello"}]}"#;
        let (transformed, original, mapped) = router.transform_request_model(body);

        assert_eq!(original, None);
        assert_eq!(mapped, None);
        // JSON 序列化可能改变键的顺序，所以我们验证内容而不是精确字符串
        let transformed_json: serde_json::Value = serde_json::from_str(&transformed).unwrap();
        let original_json: serde_json::Value = serde_json::from_str(body).unwrap();
        assert_eq!(transformed_json, original_json);
    }

    #[test]
    fn test_transform_request_model_invalid_json() {
        let router = create_test_router();

        // 测试无效 JSON
        let body = "not valid json";
        let (transformed, original, mapped) = router.transform_request_model(body);

        assert_eq!(original, None);
        assert_eq!(mapped, None);
        assert_eq!(transformed, body);
    }

    #[test]
    fn test_apply_model_mappings_batch() {
        let router = create_test_router();

        let models = vec!["claude-opus-4.5", "gpt-5", "gpt-4", "claude-sonnet-4"];
        let mapped = router.apply_model_mappings_batch(&models);

        assert_eq!(mapped[0], "claude-sonnet-4"); // 映射
        assert_eq!(mapped[1], "gemini-2.5-pro"); // 映射
        assert_eq!(mapped[2], "gpt-4"); // 无映射
        assert_eq!(mapped[3], "claude-sonnet-4"); // 无映射
    }

    #[test]
    fn test_get_reverse_mappings() {
        let mut router = create_test_router();

        // 添加另一个映射到相同目标
        router.add_model_mapping("claude-opus-4", "claude-sonnet-4");

        let reverse = router.get_reverse_mappings("claude-sonnet-4");
        assert_eq!(reverse.len(), 2);
        assert!(reverse.contains(&"claude-opus-4.5".to_string()));
        assert!(reverse.contains(&"claude-opus-4".to_string()));

        // 没有映射到的目标
        let empty = router.get_reverse_mappings("nonexistent");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_default_router() {
        let router = AmpRouter::default();

        assert!(router.upstream_url().is_none());
        assert!(router.model_mappings().is_empty());
        assert!(!router.restrict_management_to_localhost());
    }

    #[test]
    fn test_restrict_management_to_localhost() {
        let config = AmpConfig {
            upstream_url: None,
            model_mappings: vec![],
            restrict_management_to_localhost: true,
        };
        let router = AmpRouter::new(config);

        assert!(router.restrict_management_to_localhost());
    }
}
