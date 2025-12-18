//! 插件系统类型定义
//!
//! 定义 Plugin trait、PluginContext、PluginManifest 等核心类型

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

use crate::ProviderType;

/// 插件错误类型
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("插件加载失败: {0}")]
    LoadError(String),

    #[error("插件初始化失败: {0}")]
    InitError(String),

    #[error("插件执行超时: {plugin_name} 超过 {timeout_ms}ms")]
    Timeout {
        plugin_name: String,
        timeout_ms: u64,
    },

    #[error("插件执行失败: {plugin_name} - {message}")]
    ExecutionError {
        plugin_name: String,
        message: String,
    },

    #[error("插件配置错误: {0}")]
    ConfigError(String),

    #[error("插件不存在: {0}")]
    NotFound(String),

    #[error("插件已禁用: {0}")]
    Disabled(String),

    #[error("清单文件无效: {0}")]
    InvalidManifest(String),

    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON 解析错误: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// 插件状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PluginStatus {
    /// 已加载但未启用
    #[default]
    Loaded,
    /// 已启用
    Enabled,
    /// 已禁用
    Disabled,
    /// 错误状态
    Error,
}

impl fmt::Display for PluginStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginStatus::Loaded => write!(f, "loaded"),
            PluginStatus::Enabled => write!(f, "enabled"),
            PluginStatus::Disabled => write!(f, "disabled"),
            PluginStatus::Error => write!(f, "error"),
        }
    }
}

/// 插件清单 (manifest.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// 插件名称
    pub name: String,
    /// 插件版本
    pub version: String,
    /// 插件描述
    #[serde(default)]
    pub description: String,
    /// 作者
    #[serde(default)]
    pub author: Option<String>,
    /// 主页/仓库地址
    #[serde(default)]
    pub homepage: Option<String>,
    /// 许可证
    #[serde(default)]
    pub license: Option<String>,
    /// 入口文件 (相对于插件目录)
    #[serde(default = "default_entry")]
    pub entry: String,
    /// 插件类型
    #[serde(default)]
    pub plugin_type: PluginType,
    /// 配置 schema (JSON Schema)
    #[serde(default)]
    pub config_schema: Option<serde_json::Value>,
    /// 支持的钩子
    #[serde(default)]
    pub hooks: Vec<String>,
    /// 最低 ProxyCast 版本要求
    #[serde(default)]
    pub min_proxycast_version: Option<String>,
}

fn default_entry() -> String {
    "config.json".to_string()
}

impl PluginManifest {
    /// 验证清单有效性
    pub fn validate(&self) -> Result<(), PluginError> {
        if self.name.is_empty() {
            return Err(PluginError::InvalidManifest("插件名称不能为空".to_string()));
        }
        if self.version.is_empty() {
            return Err(PluginError::InvalidManifest("插件版本不能为空".to_string()));
        }
        Ok(())
    }
}

/// 插件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    /// 脚本插件 (JSON 配置驱动)
    #[default]
    #[serde(alias = "lua")]
    Script,
    /// 原生 Rust 插件 (预留)
    Native,
}

/// 插件上下文 - 传递给钩子函数的上下文信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    /// 请求 ID
    pub request_id: String,
    /// Provider 类型
    pub provider: ProviderType,
    /// 模型名称
    pub model: String,
    /// 元数据
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

impl PluginContext {
    /// 创建新的插件上下文
    pub fn new(request_id: String, provider: ProviderType, model: String) -> Self {
        Self {
            request_id,
            provider,
            model,
            metadata: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }

    /// 获取元数据
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }

    /// 设置元数据
    pub fn set_metadata(&mut self, key: &str, value: serde_json::Value) {
        self.metadata.insert(key.to_string(), value);
    }
}

/// 钩子执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    /// 是否成功
    pub success: bool,
    /// 是否修改了数据
    pub modified: bool,
    /// 错误信息 (如果失败)
    pub error: Option<String>,
    /// 执行时间 (毫秒)
    pub duration_ms: u64,
}

impl HookResult {
    /// 创建成功结果
    pub fn success(modified: bool, duration_ms: u64) -> Self {
        Self {
            success: true,
            modified,
            error: None,
            duration_ms,
        }
    }

    /// 创建失败结果
    pub fn failure(error: String, duration_ms: u64) -> Self {
        Self {
            success: false,
            modified: false,
            error: Some(error),
            duration_ms,
        }
    }
}

/// 插件配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginConfig {
    /// 插件特定配置
    #[serde(default)]
    pub settings: serde_json::Value,
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 执行超时 (毫秒)
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_enabled() -> bool {
    true
}

fn default_timeout() -> u64 {
    5000 // 5 秒
}

impl PluginConfig {
    /// 创建默认配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置配置值
    pub fn with_settings(mut self, settings: serde_json::Value) -> Self {
        self.settings = settings;
        self
    }

    /// 设置启用状态
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// 设置超时
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// 插件状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginState {
    /// 插件名称
    pub name: String,
    /// 插件状态
    pub status: PluginStatus,
    /// 加载时间
    pub loaded_at: DateTime<Utc>,
    /// 最后执行时间
    pub last_executed: Option<DateTime<Utc>>,
    /// 执行次数
    pub execution_count: u64,
    /// 错误次数
    pub error_count: u64,
    /// 最后错误信息
    pub last_error: Option<String>,
}

impl PluginState {
    /// 创建新的插件状态
    pub fn new(name: String) -> Self {
        Self {
            name,
            status: PluginStatus::Loaded,
            loaded_at: Utc::now(),
            last_executed: None,
            execution_count: 0,
            error_count: 0,
            last_error: None,
        }
    }

    /// 记录执行
    pub fn record_execution(&mut self, success: bool, error: Option<String>) {
        self.last_executed = Some(Utc::now());
        self.execution_count += 1;
        if !success {
            self.error_count += 1;
            self.last_error = error;
        }
    }
}

/// 插件信息 (用于 UI 显示)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// 插件名称
    pub name: String,
    /// 插件版本
    pub version: String,
    /// 插件描述
    pub description: String,
    /// 作者
    pub author: Option<String>,
    /// 插件状态
    pub status: PluginStatus,
    /// 插件路径
    pub path: PathBuf,
    /// 支持的钩子
    pub hooks: Vec<String>,
    /// 配置 schema
    pub config_schema: Option<serde_json::Value>,
    /// 当前配置
    pub config: PluginConfig,
    /// 运行时状态
    pub state: PluginState,
}

/// 插件 trait - 定义插件必须实现的接口
#[async_trait]
pub trait Plugin: Send + Sync {
    /// 获取插件名称
    fn name(&self) -> &str;

    /// 获取插件版本
    fn version(&self) -> &str;

    /// 获取插件清单
    fn manifest(&self) -> &PluginManifest;

    /// 初始化插件
    async fn init(&mut self, config: &PluginConfig) -> Result<(), PluginError>;

    /// 请求前钩子
    async fn on_request(
        &self,
        ctx: &mut PluginContext,
        request: &mut serde_json::Value,
    ) -> Result<HookResult, PluginError>;

    /// 响应后钩子
    async fn on_response(
        &self,
        ctx: &mut PluginContext,
        response: &mut serde_json::Value,
    ) -> Result<HookResult, PluginError>;

    /// 错误钩子
    async fn on_error(
        &self,
        ctx: &mut PluginContext,
        error: &str,
    ) -> Result<HookResult, PluginError>;

    /// 关闭插件
    async fn shutdown(&mut self) -> Result<(), PluginError>;
}

/// 插件实例包装器 - 用于管理插件生命周期
pub struct PluginInstance {
    /// 插件实现
    pub plugin: Arc<dyn Plugin>,
    /// 插件路径
    pub path: PathBuf,
    /// 插件配置
    pub config: PluginConfig,
    /// 插件状态
    pub state: PluginState,
}

impl PluginInstance {
    /// 创建新的插件实例
    pub fn new(plugin: Arc<dyn Plugin>, path: PathBuf, config: PluginConfig) -> Self {
        let state = PluginState::new(plugin.name().to_string());
        Self {
            plugin,
            path,
            config,
            state,
        }
    }

    /// 获取插件信息
    pub fn info(&self) -> PluginInfo {
        let manifest = self.plugin.manifest();
        PluginInfo {
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone(),
            author: manifest.author.clone(),
            status: self.state.status,
            path: self.path.clone(),
            hooks: manifest.hooks.clone(),
            config_schema: manifest.config_schema.clone(),
            config: self.config.clone(),
            state: self.state.clone(),
        }
    }

    /// 是否启用
    pub fn is_enabled(&self) -> bool {
        self.config.enabled && self.state.status == PluginStatus::Enabled
    }
}
