//! Management API 认证中间件
//!
//! 实现远程管理 API 的访问控制：
//! - 检查 secret_key 认证
//! - 检查 allow_remote 限制
//! - 检查 localhost 限制
//!
//! # 认证规则
//!
//! 1. 如果 secret_key 为空，返回 404 Not Found（禁用管理 API）
//! 2. 如果 allow_remote 为 false 且请求来自非 localhost，返回 403 Forbidden
//! 3. 如果请求缺少有效的 secret_key，返回 401 Unauthorized

use crate::config::RemoteManagementConfig;
use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
};
use futures::future::BoxFuture;
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    task::{Context, Poll},
};
use tower::{Layer, Service};

/// Management API 认证层
///
/// 用于包装需要认证的管理端点
#[derive(Clone)]
pub struct ManagementAuthLayer {
    config: Arc<RemoteManagementConfig>,
}

impl ManagementAuthLayer {
    /// 创建新的认证层
    pub fn new(config: RemoteManagementConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
}

impl<S> Layer<S> for ManagementAuthLayer {
    type Service = ManagementAuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ManagementAuthService {
            inner,
            config: self.config.clone(),
        }
    }
}

/// Management API 认证服务
#[derive(Clone)]
pub struct ManagementAuthService<S> {
    inner: S,
    config: Arc<RemoteManagementConfig>,
}

impl<S> ManagementAuthService<S> {
    /// 检查请求是否来自 localhost
    fn is_localhost(addr: Option<&SocketAddr>) -> bool {
        match addr {
            Some(addr) => match addr.ip() {
                IpAddr::V4(ip) => ip.is_loopback(),
                IpAddr::V6(ip) => ip.is_loopback(),
            },
            // 如果无法获取地址，保守地认为不是 localhost
            None => false,
        }
    }

    /// 从请求头中提取 secret_key
    fn extract_secret_key(req: &Request<Body>) -> Option<String> {
        // 支持两种方式：Authorization: Bearer <key> 或 X-Management-Key: <key>
        if let Some(auth) = req.headers().get("authorization") {
            if let Ok(auth_str) = auth.to_str() {
                if auth_str.starts_with("Bearer ") {
                    return Some(auth_str[7..].to_string());
                }
            }
        }

        if let Some(key) = req.headers().get("x-management-key") {
            if let Ok(key_str) = key.to_str() {
                return Some(key_str.to_string());
            }
        }

        None
    }

    /// 从请求扩展中获取客户端地址
    fn get_client_addr(req: &Request<Body>) -> Option<SocketAddr> {
        req.extensions()
            .get::<axum::extract::ConnectInfo<SocketAddr>>()
            .map(|ci| ci.0)
    }
}

impl<S> Service<Request<Body>> for ManagementAuthService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let config = self.config.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // 1. 检查 secret_key 是否为空（禁用管理 API）
            let secret_key = match &config.secret_key {
                Some(key) if !key.is_empty() => key.clone(),
                _ => {
                    tracing::debug!("[MANAGEMENT_AUTH] Management API disabled (no secret_key)");
                    return Ok(create_error_response(
                        StatusCode::NOT_FOUND,
                        "Management API is disabled",
                    ));
                }
            };

            // 2. 检查 allow_remote 限制
            let client_addr = Self::get_client_addr(&req);
            let is_localhost = Self::is_localhost(client_addr.as_ref());

            if !config.allow_remote && !is_localhost {
                tracing::warn!(
                    "[MANAGEMENT_AUTH] Remote access denied from {:?}",
                    client_addr
                );
                return Ok(create_error_response(
                    StatusCode::FORBIDDEN,
                    "Remote access is not allowed",
                ));
            }

            // 3. 验证 secret_key
            let provided_key = Self::extract_secret_key(&req);
            match provided_key {
                Some(key) if key == secret_key => {
                    // 认证成功，继续处理请求
                    tracing::debug!("[MANAGEMENT_AUTH] Auth successful from {:?}", client_addr);
                    inner.call(req).await
                }
                Some(_) => {
                    tracing::warn!(
                        "[MANAGEMENT_AUTH] Invalid secret_key from {:?}",
                        client_addr
                    );
                    Ok(create_error_response(
                        StatusCode::UNAUTHORIZED,
                        "Invalid secret key",
                    ))
                }
                None => {
                    tracing::warn!(
                        "[MANAGEMENT_AUTH] Missing secret_key from {:?}",
                        client_addr
                    );
                    Ok(create_error_response(
                        StatusCode::UNAUTHORIZED,
                        "Missing secret key",
                    ))
                }
            }
        })
    }
}

/// 创建错误响应
fn create_error_response(status: StatusCode, message: &str) -> Response<Body> {
    let body = serde_json::json!({
        "error": {
            "code": status.as_u16(),
            "message": message
        }
    });

    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_localhost_ipv4() {
        let localhost = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        assert!(ManagementAuthService::<()>::is_localhost(Some(&localhost)));

        let remote = "192.168.1.1:8080".parse::<SocketAddr>().unwrap();
        assert!(!ManagementAuthService::<()>::is_localhost(Some(&remote)));
    }

    #[test]
    fn test_is_localhost_ipv6() {
        let localhost = "[::1]:8080".parse::<SocketAddr>().unwrap();
        assert!(ManagementAuthService::<()>::is_localhost(Some(&localhost)));

        let remote = "[2001:db8::1]:8080".parse::<SocketAddr>().unwrap();
        assert!(!ManagementAuthService::<()>::is_localhost(Some(&remote)));
    }

    #[test]
    fn test_is_localhost_none() {
        assert!(!ManagementAuthService::<()>::is_localhost(None));
    }

    #[test]
    fn test_management_auth_layer_creation() {
        let config = RemoteManagementConfig {
            allow_remote: false,
            secret_key: Some("test-secret".to_string()),
            disable_control_panel: false,
        };
        let _layer = ManagementAuthLayer::new(config);
    }
}
