//! 代理客户端工厂
//!
//! 提供创建带代理配置的 HTTP 客户端的功能
//! 支持 socks5、http、https 协议

use reqwest::{Client, Proxy};
use std::time::Duration;
use thiserror::Error;

/// 代理协议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyProtocol {
    /// SOCKS5 代理
    Socks5,
    /// HTTP 代理
    Http,
    /// HTTPS 代理
    Https,
}

impl ProxyProtocol {
    /// 从 URL 字符串解析代理协议
    ///
    /// 支持的格式：
    /// - `socks5://host:port`
    /// - `http://host:port`
    /// - `https://host:port`
    pub fn from_url(url: &str) -> Option<Self> {
        let url_lower = url.to_lowercase();
        if url_lower.starts_with("socks5://") {
            Some(ProxyProtocol::Socks5)
        } else if url_lower.starts_with("http://") {
            Some(ProxyProtocol::Http)
        } else if url_lower.starts_with("https://") {
            Some(ProxyProtocol::Https)
        } else {
            None
        }
    }

    /// 获取协议名称
    pub fn as_str(&self) -> &'static str {
        match self {
            ProxyProtocol::Socks5 => "socks5",
            ProxyProtocol::Http => "http",
            ProxyProtocol::Https => "https",
        }
    }
}

/// 代理错误类型
#[derive(Debug, Error)]
pub enum ProxyError {
    /// 无效的代理 URL
    #[error("无效的代理 URL: {0}")]
    InvalidUrl(String),

    /// 不支持的代理协议
    #[error("不支持的代理协议: {0}")]
    UnsupportedProtocol(String),

    /// 代理配置错误
    #[error("代理配置错误: {0}")]
    ConfigError(String),

    /// 客户端构建错误
    #[error("客户端构建错误: {0}")]
    ClientBuildError(String),
}

/// 代理客户端工厂
///
/// 用于创建带代理配置的 HTTP 客户端
/// 支持全局代理和 Per-Key 代理
#[derive(Debug, Clone)]
pub struct ProxyClientFactory {
    /// 全局代理 URL（作为后备）
    global_proxy: Option<String>,
    /// 连接超时时间
    connect_timeout: Duration,
    /// 请求超时时间
    request_timeout: Duration,
}

impl Default for ProxyClientFactory {
    fn default() -> Self {
        Self {
            global_proxy: None,
            connect_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(300),
        }
    }
}

impl ProxyClientFactory {
    /// 创建新的代理客户端工厂
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置全局代理
    pub fn with_global_proxy(mut self, proxy_url: Option<String>) -> Self {
        self.global_proxy = proxy_url;
        self
    }

    /// 设置连接超时时间
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// 设置请求超时时间
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// 获取全局代理 URL
    pub fn global_proxy(&self) -> Option<&str> {
        self.global_proxy.as_deref()
    }

    /// 解析代理 URL 并返回协议类型
    ///
    /// # 参数
    /// - `url`: 代理 URL 字符串
    ///
    /// # 返回
    /// - `Ok(ProxyProtocol)`: 解析成功的协议类型
    /// - `Err(ProxyError)`: 解析失败的错误
    pub fn parse_proxy_url(url: &str) -> Result<ProxyProtocol, ProxyError> {
        if url.trim().is_empty() {
            return Err(ProxyError::InvalidUrl("代理 URL 不能为空".to_string()));
        }

        ProxyProtocol::from_url(url).ok_or_else(|| ProxyError::UnsupportedProtocol(url.to_string()))
    }

    /// 创建 HTTP 客户端
    ///
    /// # 参数
    /// - `per_key_proxy`: Per-Key 代理 URL（优先使用）
    ///
    /// # 返回
    /// - `Ok(Client)`: 创建成功的客户端
    /// - `Err(ProxyError)`: 创建失败的错误
    ///
    /// # 代理选择逻辑
    /// 1. 如果 `per_key_proxy` 有值，使用 Per-Key 代理
    /// 2. 否则，如果全局代理有值，使用全局代理
    /// 3. 否则，创建不带代理的客户端
    pub fn create_client(&self, per_key_proxy: Option<&str>) -> Result<Client, ProxyError> {
        // 确定要使用的代理 URL
        let proxy_url = per_key_proxy.or(self.global_proxy.as_deref());

        let mut builder = Client::builder()
            .connect_timeout(self.connect_timeout)
            .timeout(self.request_timeout);

        // 如果有代理 URL，配置代理
        if let Some(url) = proxy_url {
            let proxy = self.create_proxy(url)?;
            builder = builder.proxy(proxy);
        }

        builder
            .build()
            .map_err(|e| ProxyError::ClientBuildError(e.to_string()))
    }

    /// 创建代理配置
    fn create_proxy(&self, url: &str) -> Result<Proxy, ProxyError> {
        // 验证代理 URL 格式
        let _protocol = Self::parse_proxy_url(url)?;

        // 使用 reqwest 的 Proxy::all 来创建代理
        // 它会自动处理 socks5、http、https 协议
        Proxy::all(url).map_err(|e| ProxyError::ConfigError(e.to_string()))
    }

    /// 选择要使用的代理 URL
    ///
    /// # 参数
    /// - `per_key_proxy`: Per-Key 代理 URL
    ///
    /// # 返回
    /// - `Some(&str)`: 选择的代理 URL
    /// - `None`: 不使用代理
    pub fn select_proxy<'a>(&'a self, per_key_proxy: Option<&'a str>) -> Option<&'a str> {
        per_key_proxy.or(self.global_proxy.as_deref())
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_proxy_protocol_from_url() {
        assert_eq!(
            ProxyProtocol::from_url("socks5://127.0.0.1:1080"),
            Some(ProxyProtocol::Socks5)
        );
        assert_eq!(
            ProxyProtocol::from_url("SOCKS5://127.0.0.1:1080"),
            Some(ProxyProtocol::Socks5)
        );
        assert_eq!(
            ProxyProtocol::from_url("http://proxy.example.com:8080"),
            Some(ProxyProtocol::Http)
        );
        assert_eq!(
            ProxyProtocol::from_url("HTTP://proxy.example.com:8080"),
            Some(ProxyProtocol::Http)
        );
        assert_eq!(
            ProxyProtocol::from_url("https://secure-proxy.example.com:443"),
            Some(ProxyProtocol::Https)
        );
        assert_eq!(
            ProxyProtocol::from_url("HTTPS://secure-proxy.example.com:443"),
            Some(ProxyProtocol::Https)
        );
        assert_eq!(ProxyProtocol::from_url("ftp://invalid.com"), None);
        assert_eq!(ProxyProtocol::from_url("invalid-url"), None);
    }

    #[test]
    fn test_proxy_protocol_as_str() {
        assert_eq!(ProxyProtocol::Socks5.as_str(), "socks5");
        assert_eq!(ProxyProtocol::Http.as_str(), "http");
        assert_eq!(ProxyProtocol::Https.as_str(), "https");
    }

    #[test]
    fn test_parse_proxy_url_valid() {
        assert!(matches!(
            ProxyClientFactory::parse_proxy_url("socks5://127.0.0.1:1080"),
            Ok(ProxyProtocol::Socks5)
        ));
        assert!(matches!(
            ProxyClientFactory::parse_proxy_url("http://proxy.example.com:8080"),
            Ok(ProxyProtocol::Http)
        ));
        assert!(matches!(
            ProxyClientFactory::parse_proxy_url("https://secure-proxy.example.com:443"),
            Ok(ProxyProtocol::Https)
        ));
    }

    #[test]
    fn test_parse_proxy_url_invalid() {
        assert!(matches!(
            ProxyClientFactory::parse_proxy_url(""),
            Err(ProxyError::InvalidUrl(_))
        ));
        assert!(matches!(
            ProxyClientFactory::parse_proxy_url("   "),
            Err(ProxyError::InvalidUrl(_))
        ));
        assert!(matches!(
            ProxyClientFactory::parse_proxy_url("ftp://invalid.com"),
            Err(ProxyError::UnsupportedProtocol(_))
        ));
        assert!(matches!(
            ProxyClientFactory::parse_proxy_url("invalid-url"),
            Err(ProxyError::UnsupportedProtocol(_))
        ));
    }

    #[test]
    fn test_factory_default() {
        let factory = ProxyClientFactory::default();
        assert!(factory.global_proxy.is_none());
        assert_eq!(factory.connect_timeout, Duration::from_secs(30));
        assert_eq!(factory.request_timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_factory_with_global_proxy() {
        let factory = ProxyClientFactory::new()
            .with_global_proxy(Some("http://proxy.example.com:8080".to_string()));
        assert_eq!(
            factory.global_proxy(),
            Some("http://proxy.example.com:8080")
        );
    }

    #[test]
    fn test_factory_select_proxy() {
        // 无全局代理，无 Per-Key 代理
        let factory = ProxyClientFactory::new();
        assert_eq!(factory.select_proxy(None), None);

        // 有全局代理，无 Per-Key 代理
        let factory = ProxyClientFactory::new()
            .with_global_proxy(Some("http://global.proxy:8080".to_string()));
        assert_eq!(factory.select_proxy(None), Some("http://global.proxy:8080"));

        // 有全局代理，有 Per-Key 代理（Per-Key 优先）
        let factory = ProxyClientFactory::new()
            .with_global_proxy(Some("http://global.proxy:8080".to_string()));
        assert_eq!(
            factory.select_proxy(Some("socks5://per-key.proxy:1080")),
            Some("socks5://per-key.proxy:1080")
        );

        // 无全局代理，有 Per-Key 代理
        let factory = ProxyClientFactory::new();
        assert_eq!(
            factory.select_proxy(Some("http://per-key.proxy:8080")),
            Some("http://per-key.proxy:8080")
        );
    }

    #[test]
    fn test_create_client_no_proxy() {
        let factory = ProxyClientFactory::new();
        let client = factory.create_client(None);
        assert!(client.is_ok());
    }

    #[test]
    fn test_create_client_with_http_proxy() {
        let factory = ProxyClientFactory::new();
        let client = factory.create_client(Some("http://proxy.example.com:8080"));
        assert!(client.is_ok());
    }

    #[test]
    fn test_create_client_with_https_proxy() {
        let factory = ProxyClientFactory::new();
        let client = factory.create_client(Some("https://secure-proxy.example.com:443"));
        assert!(client.is_ok());
    }

    #[test]
    fn test_create_client_with_socks5_proxy() {
        let factory = ProxyClientFactory::new();
        let client = factory.create_client(Some("socks5://127.0.0.1:1080"));
        assert!(client.is_ok());
    }

    #[test]
    fn test_create_client_with_invalid_proxy() {
        let factory = ProxyClientFactory::new();
        let client = factory.create_client(Some("ftp://invalid.proxy:21"));
        assert!(matches!(client, Err(ProxyError::UnsupportedProtocol(_))));
    }

    #[test]
    fn test_create_client_with_global_proxy_fallback() {
        let factory = ProxyClientFactory::new()
            .with_global_proxy(Some("http://global.proxy:8080".to_string()));

        // 无 Per-Key 代理时使用全局代理
        let client = factory.create_client(None);
        assert!(client.is_ok());
    }
}
