//! 代理模块
//!
//! 提供 Per-Key 代理支持，允许为每个凭证配置独立的代理设置

mod client_factory;
#[cfg(test)]
mod tests;

pub use client_factory::{ProxyClientFactory, ProxyError, ProxyProtocol};
