//! 插件系统模块
//!
//! 提供插件扩展功能，支持：
//! - 插件加载和初始化
//! - 请求前/响应后钩子
//! - 插件隔离和错误处理
//! - 插件配置管理

mod loader;
mod manager;
mod types;

pub use loader::PluginLoader;
pub use manager::PluginManager;
pub use types::{
    HookResult, Plugin, PluginConfig, PluginContext, PluginError, PluginInfo, PluginManifest,
    PluginState, PluginStatus, PluginType,
};

#[cfg(test)]
mod tests;
