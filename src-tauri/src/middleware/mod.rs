//! Middleware 模块
//!
//! 提供 HTTP 请求处理的中间件组件

pub mod management_auth;

#[cfg(test)]
mod tests;

pub use management_auth::{ManagementAuthLayer, ManagementAuthService};
