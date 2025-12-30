---
description: 如何为 ProxyCast 添加新的 AI Provider
---

# 添加新 Provider 工作流

## 前置检查

1. 确认目标 AI 服务的认证方式（OAuth / API Key）
2. 确认 API 协议兼容性（OpenAI / Anthropic / 自定义）
3. 准备测试账号和凭证

## 步骤 1：创建 Provider 模块

在 `src-tauri/src/providers/` 创建新文件，例如 `new_service.rs`：

```rust
// src-tauri/src/providers/new_service.rs

use super::traits::Provider;
use crate::models::CredentialData;
use anyhow::Result;

pub struct NewServiceProvider {
    // 凭证数据
}

impl NewServiceProvider {
    pub fn new() -> Self {
        Self {}
    }

    // OAuth 认证方法（如需要）
    pub async fn authenticate(&mut self) -> Result<()> {
        todo!("实现认证流程")
    }

    // Token 刷新方法
    pub async fn refresh_token(&mut self) -> Result<()> {
        todo!("实现 Token 刷新")
    }
}
```

## 步骤 2：注册 Provider

在 `src-tauri/src/providers/mod.rs` 添加模块导出：

```rust
mod new_service;
pub use new_service::NewServiceProvider;
```

## 步骤 3：扩展 CredentialData 枚举

在 `src-tauri/src/models/` 相关文件中添加：

```rust
pub enum CredentialData {
    // ... 现有类型
    NewService(NewServiceCredential),
}

pub struct NewServiceCredential {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
}
```

## 步骤 4：添加健康检查

在 `src-tauri/src/services/provider_pool_service.rs` 添加：

```rust
async fn health_check_new_service(credential: &NewServiceCredential) -> bool {
    // 实现健康检查逻辑
    true
}
```

## 步骤 5：添加模型映射（如需要）

在 `src-tauri/src/converter/` 添加转换逻辑。

## 步骤 6：添加前端 UI

1. 在 `src/components/` 添加 Provider 表单组件
2. 更新设置页面

## 测试

```bash
// turbo
cd src-tauri && cargo test
```
