//! 插件系统测试

use super::*;
use crate::ProviderType;

#[test]
fn test_plugin_manifest_validation() {
    // 有效清单
    let valid = PluginManifest {
        name: "test-plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "Test plugin".to_string(),
        author: Some("Test Author".to_string()),
        homepage: None,
        license: Some("MIT".to_string()),
        entry: "config.json".to_string(),
        plugin_type: PluginType::Script,
        config_schema: None,
        hooks: vec!["on_request".to_string()],
        min_proxycast_version: None,
    };
    assert!(valid.validate().is_ok());

    // 空名称
    let invalid_name = PluginManifest {
        name: "".to_string(),
        ..valid.clone()
    };
    assert!(invalid_name.validate().is_err());

    // 空版本
    let invalid_version = PluginManifest {
        version: "".to_string(),
        ..valid.clone()
    };
    assert!(invalid_version.validate().is_err());
}

#[test]
fn test_plugin_context() {
    let mut ctx = PluginContext::new(
        "req-123".to_string(),
        ProviderType::Kiro,
        "claude-sonnet-4-5".to_string(),
    );

    assert_eq!(ctx.request_id, "req-123");
    assert_eq!(ctx.provider, ProviderType::Kiro);
    assert_eq!(ctx.model, "claude-sonnet-4-5");
    assert!(ctx.metadata.is_empty());

    // 添加元数据
    ctx.set_metadata("key1", serde_json::json!("value1"));
    assert_eq!(ctx.get_metadata("key1"), Some(&serde_json::json!("value1")));

    // 使用 builder 模式
    let ctx2 = PluginContext::new(
        "req-456".to_string(),
        ProviderType::Gemini,
        "gemini-2.5-flash".to_string(),
    )
    .with_metadata("test", serde_json::json!(123));

    assert_eq!(ctx2.get_metadata("test"), Some(&serde_json::json!(123)));
}

#[test]
fn test_hook_result() {
    let success = HookResult::success(true, 100);
    assert!(success.success);
    assert!(success.modified);
    assert!(success.error.is_none());
    assert_eq!(success.duration_ms, 100);

    let failure = HookResult::failure("test error".to_string(), 50);
    assert!(!failure.success);
    assert!(!failure.modified);
    assert_eq!(failure.error, Some("test error".to_string()));
    assert_eq!(failure.duration_ms, 50);
}

#[test]
fn test_plugin_config() {
    let config = PluginConfig::new()
        .with_enabled(true)
        .with_timeout(3000)
        .with_settings(serde_json::json!({"key": "value"}));

    assert!(config.enabled);
    assert_eq!(config.timeout_ms, 3000);
    assert_eq!(config.settings, serde_json::json!({"key": "value"}));
}

#[test]
fn test_plugin_state() {
    let mut state = PluginState::new("test-plugin".to_string());

    assert_eq!(state.name, "test-plugin");
    assert_eq!(state.status, PluginStatus::Loaded);
    assert_eq!(state.execution_count, 0);
    assert_eq!(state.error_count, 0);
    assert!(state.last_error.is_none());

    // 记录成功执行
    state.record_execution(true, None);
    assert_eq!(state.execution_count, 1);
    assert_eq!(state.error_count, 0);
    assert!(state.last_executed.is_some());

    // 记录失败执行
    state.record_execution(false, Some("test error".to_string()));
    assert_eq!(state.execution_count, 2);
    assert_eq!(state.error_count, 1);
    assert_eq!(state.last_error, Some("test error".to_string()));
}

#[test]
fn test_plugin_status_display() {
    assert_eq!(PluginStatus::Loaded.to_string(), "loaded");
    assert_eq!(PluginStatus::Enabled.to_string(), "enabled");
    assert_eq!(PluginStatus::Disabled.to_string(), "disabled");
    assert_eq!(PluginStatus::Error.to_string(), "error");
}

#[test]
fn test_plugin_error_display() {
    let err = PluginError::NotFound("test-plugin".to_string());
    assert!(err.to_string().contains("test-plugin"));

    let err = PluginError::Timeout {
        plugin_name: "slow-plugin".to_string(),
        timeout_ms: 5000,
    };
    assert!(err.to_string().contains("slow-plugin"));
    assert!(err.to_string().contains("5000"));
}

#[test]
fn test_plugin_manifest_serde() {
    let manifest = PluginManifest {
        name: "test-plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "A test plugin".to_string(),
        author: Some("Test".to_string()),
        homepage: None,
        license: None,
        entry: "config.json".to_string(),
        plugin_type: PluginType::Script,
        config_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "enabled": { "type": "boolean" }
            }
        })),
        hooks: vec!["on_request".to_string(), "on_response".to_string()],
        min_proxycast_version: Some("0.13.0".to_string()),
    };

    // 序列化
    let json = serde_json::to_string(&manifest).unwrap();
    assert!(json.contains("test-plugin"));

    // 反序列化
    let parsed: PluginManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, manifest.name);
    assert_eq!(parsed.version, manifest.version);
    assert_eq!(parsed.hooks.len(), 2);
}

// Property-based tests
use proptest::prelude::*;

/// **Feature: enhancement-roadmap, Property 17: 插件隔离性**
/// **Validates: Requirements 6.3 (验收标准 3)**
///
/// *对于任意* 插件执行失败，主请求处理流程应继续正常执行
mod property_tests {
    use super::*;
    use crate::plugin::manager::{PluginManager, PluginManagerConfig};
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// 生成随机的请求 JSON
    fn arb_request() -> impl Strategy<Value = serde_json::Value> {
        prop::collection::hash_map(
            "[a-z]{1,10}",
            prop_oneof![
                Just(serde_json::Value::Null),
                any::<bool>().prop_map(serde_json::Value::Bool),
                any::<i64>().prop_map(|n| serde_json::Value::Number(n.into())),
                "[a-zA-Z0-9 ]{0,50}".prop_map(serde_json::Value::String),
            ],
            0..5,
        )
        .prop_map(|map| serde_json::Value::Object(map.into_iter().collect()))
    }

    /// 生成随机的插件上下文
    fn arb_context() -> impl Strategy<Value = PluginContext> {
        (
            "[a-z0-9]{8,16}", // request_id
            prop_oneof![
                Just(ProviderType::Kiro),
                Just(ProviderType::Gemini),
                Just(ProviderType::Qwen),
            ],
            "[a-z0-9-]{5,20}", // model
        )
            .prop_map(|(request_id, provider, model)| {
                PluginContext::new(request_id, provider, model)
            })
    }

    proptest! {
        /// **Property 17: 插件隔离性**
        ///
        /// 验证：即使没有插件加载，PluginManager 的钩子执行也应该正常完成
        /// 不会因为空插件列表而失败
        #[test]
        fn plugin_manager_handles_empty_plugins(
            request in arb_request(),
            ctx in arb_context()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let (results_empty, req_unchanged) = rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let config = PluginManagerConfig {
                    default_timeout_ms: 1000,
                    enabled: true,
                    max_plugins: 10,
                };
                let manager = PluginManager::new(temp_dir.path().to_path_buf(), config);

                let mut ctx = ctx;
                let mut req = request.clone();

                // 即使没有插件，钩子执行也应该成功返回空结果
                let results = manager.run_on_request(&mut ctx, &mut req).await;

                (results.is_empty(), req == request)
            });

            prop_assert!(results_empty, "Empty plugin list should return empty results");
            prop_assert!(req_unchanged, "Request should not be modified when no plugins");
        }

        /// **Property 17: 插件隔离性 - 禁用状态**
        ///
        /// 验证：当插件系统禁用时，钩子执行应该正常完成且不修改数据
        #[test]
        fn disabled_plugin_system_does_not_affect_request(
            request in arb_request(),
            ctx in arb_context()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let (results_empty, req_unchanged) = rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let config = PluginManagerConfig {
                    default_timeout_ms: 1000,
                    enabled: false,  // 禁用插件系统
                    max_plugins: 10,
                };
                let manager = PluginManager::new(temp_dir.path().to_path_buf(), config);

                let mut ctx = ctx;
                let mut req = request.clone();

                // 禁用状态下，钩子执行应该返回空结果
                let results = manager.run_on_request(&mut ctx, &mut req).await;

                (results.is_empty(), req == request)
            });

            prop_assert!(results_empty, "Disabled plugin system should return empty results");
            prop_assert!(req_unchanged, "Request should not be modified when plugins disabled");
        }
    }
}
