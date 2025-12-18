//! 路由系统模块
//!
//! 支持动态路由注册、命名空间路由解析、模型映射和路由规则。
//!
//! 路由格式：
//! - `/{provider-name}/v1/messages` - Provider 命名空间路由
//! - `/{selector}/v1/messages` - 凭证选择器路由（向后兼容）
//! - `/v1/messages` - 默认路由
//! - `/api/provider/{provider}/v1/*` - Amp CLI 路由
//!
//! 模型映射：
//! - 支持模型别名映射（如 `gpt-4` -> `claude-sonnet-4-5-20250514`）
//!
//! 路由规则：
//! - 支持通配符模式匹配（前缀、后缀、包含）
//! - 支持规则优先级排序

mod amp_router;
mod mapper;
mod provider_router;
mod route_registry;
mod rules;

pub use amp_router::{AmpRouteMatch, AmpRouter};
pub use mapper::{ModelInfo, ModelMapper};
pub use provider_router::ProviderRouter;
pub use route_registry::{RegisteredRoute, RouteRegistry, RouteType};
pub use rules::{RouteResult, Router, RoutingRule};

#[cfg(test)]
mod tests;
