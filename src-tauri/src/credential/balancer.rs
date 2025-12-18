//! 负载均衡器实现
//!
//! 提供轮询负载均衡策略，支持凭证冷却和自动恢复

use super::health::{HealthCheckConfig, HealthChecker};
use super::pool::{CredentialPool, PoolError};
use super::types::Credential;
use crate::proxy::ProxyClientFactory;
use crate::ProviderType;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// 负载均衡策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BalanceStrategy {
    /// 轮询策略（默认）
    #[default]
    RoundRobin,
    /// 最少使用策略
    LeastUsed,
    /// 随机策略
    Random,
}

/// 冷却信息
#[derive(Debug, Clone)]
pub struct CooldownInfo {
    /// 冷却结束时间
    pub until: DateTime<Utc>,
    /// 冷却原因
    pub reason: String,
}

/// 凭证选择结果 - 包含凭证和对应的 HTTP 客户端
#[derive(Debug)]
pub struct CredentialSelection {
    /// 选中的凭证
    pub credential: Credential,
    /// 配置了代理的 HTTP 客户端
    pub client: Client,
}

/// 负载均衡器 - 管理多个 Provider 的凭证池
pub struct LoadBalancer {
    /// 负载均衡策略
    strategy: BalanceStrategy,
    /// 各 Provider 的凭证池
    pools: DashMap<ProviderType, Arc<CredentialPool>>,
    /// 轮询索引（每个 Provider 独立）
    round_robin_indices: DashMap<ProviderType, AtomicUsize>,
    /// 健康检查器
    health_checker: HealthChecker,
    /// 代理客户端工厂
    proxy_factory: ProxyClientFactory,
}

impl LoadBalancer {
    /// 创建新的负载均衡器
    pub fn new(strategy: BalanceStrategy) -> Self {
        Self {
            strategy,
            pools: DashMap::new(),
            round_robin_indices: DashMap::new(),
            health_checker: HealthChecker::with_defaults(),
            proxy_factory: ProxyClientFactory::new(),
        }
    }

    /// 创建使用轮询策略的负载均衡器
    pub fn round_robin() -> Self {
        Self::new(BalanceStrategy::RoundRobin)
    }

    /// 创建带自定义健康检查配置的负载均衡器
    pub fn with_health_config(strategy: BalanceStrategy, health_config: HealthCheckConfig) -> Self {
        Self {
            strategy,
            pools: DashMap::new(),
            round_robin_indices: DashMap::new(),
            health_checker: HealthChecker::new(health_config),
            proxy_factory: ProxyClientFactory::new(),
        }
    }

    /// 创建带全局代理的负载均衡器
    pub fn with_global_proxy(mut self, proxy_url: Option<String>) -> Self {
        self.proxy_factory = self.proxy_factory.with_global_proxy(proxy_url);
        self
    }

    /// 设置全局代理
    pub fn set_global_proxy(&mut self, proxy_url: Option<String>) {
        self.proxy_factory = ProxyClientFactory::new().with_global_proxy(proxy_url);
    }

    /// 获取代理客户端工厂
    pub fn proxy_factory(&self) -> &ProxyClientFactory {
        &self.proxy_factory
    }

    /// 获取健康检查器
    pub fn health_checker(&self) -> &HealthChecker {
        &self.health_checker
    }

    /// 获取当前策略
    pub fn strategy(&self) -> BalanceStrategy {
        self.strategy
    }

    /// 设置负载均衡策略
    pub fn set_strategy(&mut self, strategy: BalanceStrategy) {
        self.strategy = strategy;
    }

    /// 注册凭证池
    pub fn register_pool(&self, pool: Arc<CredentialPool>) {
        let provider = pool.provider();
        self.pools.insert(provider, pool);
        self.round_robin_indices
            .insert(provider, AtomicUsize::new(0));
    }

    /// 获取凭证池
    pub fn get_pool(&self, provider: ProviderType) -> Option<Arc<CredentialPool>> {
        self.pools.get(&provider).map(|r| r.value().clone())
    }

    /// 移除凭证池
    pub fn remove_pool(&self, provider: ProviderType) -> Option<Arc<CredentialPool>> {
        self.round_robin_indices.remove(&provider);
        self.pools.remove(&provider).map(|(_, pool)| pool)
    }

    /// 获取所有已注册的 Provider
    pub fn providers(&self) -> Vec<ProviderType> {
        self.pools.iter().map(|r| *r.key()).collect()
    }

    /// 选择下一个可用凭证（使用当前策略）
    ///
    /// # 错误
    /// - 如果 Provider 未注册，返回 `PoolError::EmptyPool`
    /// - 如果没有可用凭证，返回 `PoolError::NoAvailableCredential`
    pub fn select(&self, provider: ProviderType) -> Result<Credential, PoolError> {
        let pool = self.pools.get(&provider).ok_or(PoolError::EmptyPool)?;

        // 先刷新冷却状态
        pool.refresh_cooldowns();

        match self.strategy {
            BalanceStrategy::RoundRobin => self.select_round_robin(&pool, provider),
            BalanceStrategy::LeastUsed => self.select_least_used(&pool),
            BalanceStrategy::Random => self.select_random(&pool),
        }
    }

    /// 选择下一个可用凭证并创建配置了代理的 HTTP 客户端
    ///
    /// 代理选择逻辑：
    /// 1. 如果凭证有 proxy_url，使用 Per-Key 代理
    /// 2. 否则，使用全局代理（如果配置了）
    /// 3. 否则，不使用代理
    ///
    /// # 错误
    /// - 如果 Provider 未注册，返回 `PoolError::EmptyPool`
    /// - 如果没有可用凭证，返回 `PoolError::NoAvailableCredential`
    /// - 如果代理配置无效，返回 `PoolError::CredentialNotFound`（包含错误信息）
    pub fn select_with_client(
        &self,
        provider: ProviderType,
    ) -> Result<CredentialSelection, PoolError> {
        let credential = self.select(provider)?;

        // 使用凭证的 proxy_url 或回退到全局代理
        let client = self
            .proxy_factory
            .create_client(credential.proxy_url())
            .map_err(|e| PoolError::CredentialNotFound(format!("代理配置错误: {}", e)))?;

        Ok(CredentialSelection { credential, client })
    }

    /// 为指定凭证创建配置了代理的 HTTP 客户端
    ///
    /// # 参数
    /// - `credential`: 凭证引用
    ///
    /// # 返回
    /// - `Ok(Client)`: 配置了代理的 HTTP 客户端
    /// - `Err(PoolError)`: 代理配置错误
    pub fn create_client_for_credential(
        &self,
        credential: &Credential,
    ) -> Result<Client, PoolError> {
        self.proxy_factory
            .create_client(credential.proxy_url())
            .map_err(|e| PoolError::CredentialNotFound(format!("代理配置错误: {}", e)))
    }

    /// 选择下一个可用凭证，支持代理失败时的故障转移
    ///
    /// 当代理连接失败时，自动尝试下一个可用凭证。
    /// 最多尝试 `max_attempts` 次（默认为池中凭证数量）。
    ///
    /// # 参数
    /// - `provider`: Provider 类型
    /// - `max_attempts`: 最大尝试次数（None 表示尝试所有可用凭证）
    ///
    /// # 返回
    /// - `Ok(CredentialSelection)`: 成功选择的凭证和客户端
    /// - `Err(PoolError)`: 所有凭证都失败
    pub fn select_with_failover(
        &self,
        provider: ProviderType,
        max_attempts: Option<usize>,
    ) -> Result<CredentialSelection, PoolError> {
        let pool = self.pools.get(&provider).ok_or(PoolError::EmptyPool)?;
        pool.refresh_cooldowns();

        let active_count = pool.active_count();
        if active_count == 0 {
            return Err(PoolError::NoAvailableCredential);
        }

        let attempts = max_attempts.unwrap_or(active_count).min(active_count);
        let mut last_error = None;
        let mut tried_ids = std::collections::HashSet::new();

        for _ in 0..attempts {
            // 选择下一个凭证
            let credential = match self.select(provider) {
                Ok(cred) => cred,
                Err(e) => {
                    last_error = Some(e);
                    break;
                }
            };

            // 避免重复尝试同一个凭证
            if tried_ids.contains(&credential.id) {
                continue;
            }
            tried_ids.insert(credential.id.clone());

            // 尝试创建客户端
            match self.proxy_factory.create_client(credential.proxy_url()) {
                Ok(client) => {
                    return Ok(CredentialSelection { credential, client });
                }
                Err(e) => {
                    // 记录警告并继续尝试下一个凭证
                    tracing::warn!(
                        credential_id = %credential.id,
                        proxy_url = ?credential.proxy_url(),
                        error = %e,
                        "代理连接失败，尝试下一个凭证"
                    );
                    last_error = Some(PoolError::CredentialNotFound(format!(
                        "凭证 {} 的代理配置错误: {}",
                        credential.id, e
                    )));
                }
            }
        }

        Err(last_error.unwrap_or(PoolError::NoAvailableCredential))
    }

    /// 报告代理连接失败并尝试故障转移
    ///
    /// 当代理连接失败时调用此方法，它会：
    /// 1. 记录失败
    /// 2. 尝试选择下一个可用凭证
    ///
    /// # 参数
    /// - `provider`: Provider 类型
    /// - `failed_credential_id`: 失败的凭证 ID
    ///
    /// # 返回
    /// - `Ok(CredentialSelection)`: 故障转移成功，返回新的凭证和客户端
    /// - `Err(PoolError)`: 故障转移失败
    pub fn failover_on_proxy_error(
        &self,
        provider: ProviderType,
        failed_credential_id: &str,
    ) -> Result<CredentialSelection, PoolError> {
        // 记录失败
        let _ = self.report(provider, failed_credential_id, false, 0);

        tracing::warn!(
            credential_id = %failed_credential_id,
            provider = %provider,
            "代理连接失败，执行故障转移"
        );

        // 尝试选择下一个凭证
        self.select_with_client(provider)
    }

    /// 轮询选择凭证
    fn select_round_robin(
        &self,
        pool: &CredentialPool,
        provider: ProviderType,
    ) -> Result<Credential, PoolError> {
        // 收集所有活跃凭证
        let active_creds: Vec<Credential> = pool
            .all()
            .into_iter()
            .filter(|c| c.is_available())
            .collect();

        if active_creds.is_empty() {
            return Err(PoolError::NoAvailableCredential);
        }

        // 获取或创建轮询索引
        let index_entry = self
            .round_robin_indices
            .entry(provider)
            .or_insert_with(|| AtomicUsize::new(0));

        // 原子递增并取模
        let index = index_entry.fetch_add(1, Ordering::SeqCst) % active_creds.len();
        Ok(active_creds[index].clone())
    }

    /// 最少使用选择凭证
    fn select_least_used(&self, pool: &CredentialPool) -> Result<Credential, PoolError> {
        pool.all()
            .into_iter()
            .filter(|c| c.is_available())
            .min_by_key(|c| c.stats.total_requests)
            .ok_or(PoolError::NoAvailableCredential)
    }

    /// 随机选择凭证
    fn select_random(&self, pool: &CredentialPool) -> Result<Credential, PoolError> {
        let active_creds: Vec<Credential> = pool
            .all()
            .into_iter()
            .filter(|c| c.is_available())
            .collect();

        if active_creds.is_empty() {
            return Err(PoolError::NoAvailableCredential);
        }

        // 使用简单的伪随机（基于时间戳）
        let now = Utc::now().timestamp_nanos_opt().unwrap_or(0) as usize;
        let index = now % active_creds.len();
        Ok(active_creds[index].clone())
    }

    /// 标记凭证为冷却状态
    ///
    /// # 参数
    /// - `provider`: Provider 类型
    /// - `credential_id`: 凭证 ID
    /// - `duration`: 冷却时长
    ///
    /// # 错误
    /// - 如果 Provider 未注册或凭证不存在
    pub fn mark_cooldown(
        &self,
        provider: ProviderType,
        credential_id: &str,
        duration: Duration,
    ) -> Result<(), PoolError> {
        let pool = self.pools.get(&provider).ok_or(PoolError::EmptyPool)?;

        pool.mark_cooldown(credential_id, duration)
    }

    /// 恢复凭证为活跃状态
    pub fn mark_active(
        &self,
        provider: ProviderType,
        credential_id: &str,
    ) -> Result<(), PoolError> {
        let pool = self.pools.get(&provider).ok_or(PoolError::EmptyPool)?;

        pool.mark_active(credential_id)
    }

    /// 刷新所有池的冷却状态
    pub fn refresh_all_cooldowns(&self) {
        for pool in self.pools.iter() {
            pool.refresh_cooldowns();
        }
    }

    /// 报告凭证使用结果
    ///
    /// 自动更新健康状态：
    /// - 成功时：如果凭证之前不健康，恢复为健康
    /// - 失败时：如果连续失败达到阈值（默认 3 次），标记为不健康
    ///
    /// # 参数
    /// - `provider`: Provider 类型
    /// - `credential_id`: 凭证 ID
    /// - `success`: 是否成功
    /// - `latency_ms`: 延迟（毫秒）
    ///
    /// # 返回
    /// - `Ok(true)` 如果健康状态发生变化
    /// - `Ok(false)` 如果健康状态未变化
    pub fn report(
        &self,
        provider: ProviderType,
        credential_id: &str,
        success: bool,
        latency_ms: u64,
    ) -> Result<bool, PoolError> {
        let pool = self.pools.get(&provider).ok_or(PoolError::EmptyPool)?;

        if success {
            self.health_checker
                .record_success(&pool, credential_id, latency_ms)
        } else {
            self.health_checker.record_failure(&pool, credential_id)
        }
    }

    /// 获取 Provider 的最早恢复时间
    pub fn earliest_recovery(&self, provider: ProviderType) -> Option<DateTime<Utc>> {
        self.pools
            .get(&provider)
            .and_then(|pool| pool.earliest_recovery())
    }

    /// 检查是否有可用凭证
    pub fn has_available(&self, provider: ProviderType) -> bool {
        self.pools
            .get(&provider)
            .map(|pool| pool.active_count() > 0)
            .unwrap_or(false)
    }

    /// 获取活跃凭证数量
    pub fn active_count(&self, provider: ProviderType) -> usize {
        self.pools
            .get(&provider)
            .map(|pool| pool.active_count())
            .unwrap_or(0)
    }
}

impl Default for LoadBalancer {
    fn default() -> Self {
        Self::round_robin()
    }
}

#[cfg(test)]
mod balancer_tests {
    use super::*;
    use crate::credential::CredentialData;

    fn create_test_credential(id: &str, provider: ProviderType) -> Credential {
        Credential::new(
            id.to_string(),
            provider,
            CredentialData::ApiKey {
                key: format!("key-{}", id),
                base_url: None,
            },
        )
    }

    #[test]
    fn test_load_balancer_new() {
        let lb = LoadBalancer::new(BalanceStrategy::RoundRobin);
        assert_eq!(lb.strategy(), BalanceStrategy::RoundRobin);
        assert!(lb.providers().is_empty());
    }

    #[test]
    fn test_load_balancer_register_pool() {
        let lb = LoadBalancer::round_robin();
        let pool = Arc::new(CredentialPool::new(ProviderType::Kiro));
        pool.add(create_test_credential("cred-1", ProviderType::Kiro))
            .unwrap();

        lb.register_pool(pool.clone());

        assert!(lb.providers().contains(&ProviderType::Kiro));
        assert!(lb.get_pool(ProviderType::Kiro).is_some());
    }

    #[test]
    fn test_load_balancer_select_round_robin() {
        let lb = LoadBalancer::round_robin();
        let pool = Arc::new(CredentialPool::new(ProviderType::Kiro));

        pool.add(create_test_credential("cred-1", ProviderType::Kiro))
            .unwrap();
        pool.add(create_test_credential("cred-2", ProviderType::Kiro))
            .unwrap();
        pool.add(create_test_credential("cred-3", ProviderType::Kiro))
            .unwrap();

        lb.register_pool(pool);

        // 轮询应该依次返回不同的凭证
        let c1 = lb.select(ProviderType::Kiro).unwrap();
        let c2 = lb.select(ProviderType::Kiro).unwrap();
        let c3 = lb.select(ProviderType::Kiro).unwrap();

        // 三次选择应该返回三个不同的凭证
        let ids: std::collections::HashSet<_> = [c1.id, c2.id, c3.id].into_iter().collect();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn test_load_balancer_select_empty_pool() {
        let lb = LoadBalancer::round_robin();
        let result = lb.select(ProviderType::Kiro);
        assert!(matches!(result, Err(PoolError::EmptyPool)));
    }

    #[test]
    fn test_load_balancer_cooldown() {
        let lb = LoadBalancer::round_robin();
        let pool = Arc::new(CredentialPool::new(ProviderType::Kiro));

        pool.add(create_test_credential("cred-1", ProviderType::Kiro))
            .unwrap();
        pool.add(create_test_credential("cred-2", ProviderType::Kiro))
            .unwrap();

        lb.register_pool(pool);

        // 标记 cred-1 为冷却
        lb.mark_cooldown(ProviderType::Kiro, "cred-1", Duration::hours(1))
            .unwrap();

        // 现在只有 cred-2 可用
        assert_eq!(lb.active_count(ProviderType::Kiro), 1);

        // 选择应该只返回 cred-2
        let selected = lb.select(ProviderType::Kiro).unwrap();
        assert_eq!(selected.id, "cred-2");
    }

    #[test]
    fn test_load_balancer_report() {
        let lb = LoadBalancer::round_robin();
        let pool = Arc::new(CredentialPool::new(ProviderType::Kiro));

        pool.add(create_test_credential("cred-1", ProviderType::Kiro))
            .unwrap();

        lb.register_pool(pool.clone());

        // 报告成功
        let changed = lb.report(ProviderType::Kiro, "cred-1", true, 100).unwrap();
        assert!(!changed); // 状态未变化

        let cred = pool.get("cred-1").unwrap();
        assert_eq!(cred.stats.total_requests, 1);
        assert_eq!(cred.stats.successful_requests, 1);

        // 报告失败
        let changed = lb.report(ProviderType::Kiro, "cred-1", false, 0).unwrap();
        assert!(!changed); // 第一次失败，状态未变化

        let cred = pool.get("cred-1").unwrap();
        assert_eq!(cred.stats.total_requests, 2);
        assert_eq!(cred.stats.consecutive_failures, 1);
    }

    #[test]
    fn test_load_balancer_auto_unhealthy() {
        use crate::credential::CredentialStatus;

        let lb = LoadBalancer::round_robin();
        let pool = Arc::new(CredentialPool::new(ProviderType::Kiro));

        pool.add(create_test_credential("cred-1", ProviderType::Kiro))
            .unwrap();

        lb.register_pool(pool.clone());

        // 连续 3 次失败应标记为不健康
        assert!(!lb.report(ProviderType::Kiro, "cred-1", false, 0).unwrap());
        assert!(!lb.report(ProviderType::Kiro, "cred-1", false, 0).unwrap());
        assert!(lb.report(ProviderType::Kiro, "cred-1", false, 0).unwrap()); // 第 3 次应返回 true

        let cred = pool.get("cred-1").unwrap();
        assert!(matches!(cred.status, CredentialStatus::Unhealthy { .. }));
    }

    #[test]
    fn test_load_balancer_auto_recovery() {
        use crate::credential::CredentialStatus;

        let lb = LoadBalancer::round_robin();
        let pool = Arc::new(CredentialPool::new(ProviderType::Kiro));

        pool.add(create_test_credential("cred-1", ProviderType::Kiro))
            .unwrap();

        lb.register_pool(pool.clone());

        // 先标记为不健康
        pool.mark_unhealthy("cred-1", "test".to_string()).unwrap();

        // 成功后应恢复
        let recovered = lb.report(ProviderType::Kiro, "cred-1", true, 100).unwrap();
        assert!(recovered);

        let cred = pool.get("cred-1").unwrap();
        assert!(matches!(cred.status, CredentialStatus::Active));
    }

    #[test]
    fn test_load_balancer_cooldown_recovery() {
        let lb = LoadBalancer::round_robin();
        let pool = Arc::new(CredentialPool::new(ProviderType::Kiro));

        pool.add(create_test_credential("cred-1", ProviderType::Kiro))
            .unwrap();

        lb.register_pool(pool.clone());

        // 标记为已过期的冷却（使用负时长模拟过期）
        // 直接设置状态为过去的时间
        {
            let mut entry = pool.credentials.get_mut("cred-1").unwrap();
            entry.status = crate::credential::CredentialStatus::Cooldown {
                until: Utc::now() - Duration::seconds(1),
            };
        }

        // 此时凭证应该处于冷却状态
        let cred = pool.get("cred-1").unwrap();
        assert!(matches!(
            cred.status,
            crate::credential::CredentialStatus::Cooldown { .. }
        ));

        // 调用 select 会触发 refresh_cooldowns，应该自动恢复
        let selected = lb.select(ProviderType::Kiro).unwrap();
        assert_eq!(selected.id, "cred-1");

        // 验证状态已恢复为 Active
        let cred = pool.get("cred-1").unwrap();
        assert!(matches!(
            cred.status,
            crate::credential::CredentialStatus::Active
        ));
    }

    #[test]
    fn test_load_balancer_earliest_recovery() {
        let lb = LoadBalancer::round_robin();
        let pool = Arc::new(CredentialPool::new(ProviderType::Kiro));

        pool.add(create_test_credential("cred-1", ProviderType::Kiro))
            .unwrap();
        pool.add(create_test_credential("cred-2", ProviderType::Kiro))
            .unwrap();

        lb.register_pool(pool);

        // 没有冷却时应返回 None
        assert!(lb.earliest_recovery(ProviderType::Kiro).is_none());

        // 标记两个凭证为不同的冷却时间
        lb.mark_cooldown(ProviderType::Kiro, "cred-1", Duration::hours(2))
            .unwrap();
        lb.mark_cooldown(ProviderType::Kiro, "cred-2", Duration::hours(1))
            .unwrap();

        // 应返回最早的恢复时间（cred-2 的 1 小时后）
        let recovery = lb.earliest_recovery(ProviderType::Kiro);
        assert!(recovery.is_some());

        // 验证恢复时间大约在 1 小时后（允许几秒误差）
        let expected = Utc::now() + Duration::hours(1);
        let diff = (recovery.unwrap() - expected).num_seconds().abs();
        assert!(
            diff < 5,
            "Recovery time should be approximately 1 hour from now"
        );
    }
}
