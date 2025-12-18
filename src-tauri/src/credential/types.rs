//! 凭证相关类型定义
//!
//! 定义凭证、凭证数据、凭证状态等核心类型

use crate::ProviderType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 凭证 - 表示单个 API 凭证
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Credential {
    /// 唯一标识符
    pub id: String,
    /// 所属 Provider 类型
    pub provider: ProviderType,
    /// 凭证数据
    pub data: CredentialData,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后使用时间
    pub last_used: Option<DateTime<Utc>>,
    /// 当前状态
    pub status: CredentialStatus,
    /// 统计信息
    pub stats: CredentialStats,
    /// Per-Key 代理 URL（覆盖全局代理）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
}

impl Credential {
    /// 创建新凭证
    pub fn new(id: String, provider: ProviderType, data: CredentialData) -> Self {
        Self {
            id,
            provider,
            data,
            created_at: Utc::now(),
            last_used: None,
            status: CredentialStatus::Active,
            stats: CredentialStats::default(),
            proxy_url: None,
        }
    }

    /// 创建带代理的凭证
    pub fn with_proxy(mut self, proxy_url: Option<String>) -> Self {
        self.proxy_url = proxy_url;
        self
    }

    /// 设置代理 URL
    pub fn set_proxy_url(&mut self, proxy_url: Option<String>) {
        self.proxy_url = proxy_url;
    }

    /// 获取代理 URL
    pub fn proxy_url(&self) -> Option<&str> {
        self.proxy_url.as_deref()
    }

    /// 检查凭证是否可用（活跃状态）
    pub fn is_available(&self) -> bool {
        matches!(self.status, CredentialStatus::Active)
    }

    /// 更新最后使用时间
    pub fn mark_used(&mut self) {
        self.last_used = Some(Utc::now());
    }
}

/// 凭证数据 - 不同 Provider 有不同的凭证格式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CredentialData {
    /// OAuth 凭证（用于 Kiro、Gemini、Qwen 等）
    OAuth {
        access_token: String,
        refresh_token: Option<String>,
        expires_at: Option<DateTime<Utc>>,
    },
    /// API Key 凭证（用于 OpenAI、Claude 等）
    ApiKey {
        key: String,
        base_url: Option<String>,
    },
}

/// 凭证状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CredentialStatus {
    /// 活跃可用
    Active,
    /// 冷却中（配额超限等）
    Cooldown {
        /// 冷却结束时间
        until: DateTime<Utc>,
    },
    /// 不健康（连续失败）
    Unhealthy {
        /// 不健康原因
        reason: String,
    },
    /// 已禁用
    Disabled,
}

/// 凭证统计信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CredentialStats {
    /// 总请求数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 连续失败次数
    pub consecutive_failures: u32,
    /// 平均延迟（毫秒）
    pub avg_latency_ms: f64,
}

impl CredentialStats {
    /// 记录成功请求
    pub fn record_success(&mut self, latency_ms: u64) {
        self.total_requests += 1;
        self.successful_requests += 1;
        self.consecutive_failures = 0;

        // 更新平均延迟（移动平均）
        let n = self.successful_requests as f64;
        self.avg_latency_ms = self.avg_latency_ms * (n - 1.0) / n + latency_ms as f64 / n;
    }

    /// 记录失败请求
    pub fn record_failure(&mut self) {
        self.total_requests += 1;
        self.consecutive_failures += 1;
    }

    /// 获取成功率
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            1.0
        } else {
            self.successful_requests as f64 / self.total_requests as f64
        }
    }
}

#[cfg(test)]
mod type_tests {
    use super::*;

    #[test]
    fn test_credential_new() {
        let cred = Credential::new(
            "test-id".to_string(),
            ProviderType::Kiro,
            CredentialData::ApiKey {
                key: "test-key".to_string(),
                base_url: None,
            },
        );

        assert_eq!(cred.id, "test-id");
        assert_eq!(cred.provider, ProviderType::Kiro);
        assert!(cred.is_available());
        assert!(cred.last_used.is_none());
    }

    #[test]
    fn test_credential_is_available() {
        let mut cred = Credential::new(
            "test".to_string(),
            ProviderType::Gemini,
            CredentialData::ApiKey {
                key: "key".to_string(),
                base_url: None,
            },
        );

        assert!(cred.is_available());

        cred.status = CredentialStatus::Cooldown {
            until: Utc::now() + chrono::Duration::hours(1),
        };
        assert!(!cred.is_available());

        cred.status = CredentialStatus::Unhealthy {
            reason: "test".to_string(),
        };
        assert!(!cred.is_available());

        cred.status = CredentialStatus::Disabled;
        assert!(!cred.is_available());
    }

    #[test]
    fn test_credential_stats_success() {
        let mut stats = CredentialStats::default();

        stats.record_success(100);
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.successful_requests, 1);
        assert_eq!(stats.consecutive_failures, 0);
        assert!((stats.avg_latency_ms - 100.0).abs() < 0.001);

        stats.record_success(200);
        assert_eq!(stats.total_requests, 2);
        assert!((stats.avg_latency_ms - 150.0).abs() < 0.001);
    }

    #[test]
    fn test_credential_stats_failure() {
        let mut stats = CredentialStats::default();

        stats.record_failure();
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.successful_requests, 0);
        assert_eq!(stats.consecutive_failures, 1);

        stats.record_failure();
        assert_eq!(stats.consecutive_failures, 2);

        stats.record_success(100);
        assert_eq!(stats.consecutive_failures, 0);
    }

    #[test]
    fn test_credential_stats_success_rate() {
        let mut stats = CredentialStats::default();

        // 空统计应返回 1.0
        assert!((stats.success_rate() - 1.0).abs() < 0.001);

        stats.record_success(100);
        assert!((stats.success_rate() - 1.0).abs() < 0.001);

        stats.record_failure();
        assert!((stats.success_rate() - 0.5).abs() < 0.001);
    }
}
