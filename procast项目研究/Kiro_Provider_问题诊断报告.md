# ProxyCast Kiro Provider 问题诊断与分析报告

> **创建日期**: 2025-12-29  
> **问题类型**: Kiro OAuth 凭证请求失败  
> **涉及模块**: 凭证池管理、Token 刷新、API 路由

---

## 目录

1. [问题描述](#1-问题描述)
2. [错误分析](#2-错误分析)
3. [涉及模块分析](#3-涉及模块分析)
4. [Token 刷新机制](#4-token-刷新机制)
5. [解决方案与建议](#5-解决方案与建议)

---

## 1. 问题描述

### 1.1 现象

- Claude Code 发送请求后显示 "Effecting... 0 tokens"，持续约 65 秒后失败
- Flow Monitor 显示多个请求状态为 "Failed"，状态码 500
- 凭证池显示 Kiro 凭证错误信息："error decoding response body"

### 1.2 测试对比

| 请求类型 | 结果 | 耗时 | 说明 |
|---------|------|------|------|
| 简单 API 测试 "Say hi" | ✅ 成功 | 49秒 | 10 tokens 返回 |
| Claude Code 完整请求 | ❌ 失败 | 65秒超时 | 包含系统提示和工具定义 |

---

## 2. 错误分析

### 2.1 错误日志

从 Flow 数据库查询到的失败请求详情：

```json
{
  "error_type": "network",
  "message": "error decoding response body",
  "retryable": false
}
```

### 2.2 错误时间线

```
2025-12-29T01:51:20  ❌ Failed (65.5s)
2025-12-29T01:52:26  ❌ Failed (65.2s)
2025-12-29T01:53:33  ❌ Failed (65.2s)
2025-12-29T01:54:40  ❌ Failed (65.7s)
2025-12-29T01:55:00  ✅ Completed (49.4s) - 简单测试请求
```

### 2.3 根本原因分析

**"error decoding response body"** 表明：

1. **Kiro 服务端返回了无法解析的响应格式**
2. **可能原因**：
   - Claude Code 的完整请求体过大（包含系统提示、工具定义等）
   - Kiro SSE 流式响应在处理大请求时格式异常
   - Kiro 服务端对长时间请求的处理超时

---

## 3. 涉及模块分析

### 3.1 凭证池管理模块

**位置**: `src-tauri/src/services/provider_pool_service.rs`

**核心功能**：
- 凭证的 CRUD 操作
- 智能轮换策略（基于权重评分）
- 健康检查和状态管理
- Token 刷新

**关键函数**：

```rust
// 选择最优凭证（智能轮换）
pub fn select_credential(&self, db, provider_type, model) -> Result<Option<ProviderCredential>>

// 计算凭证分数（健康度40分、使用频率30分、错误率20分、冷却时间10分）
fn calculate_credential_score(&self, cred, now, all_credentials) -> f64

// 执行健康检查
pub async fn check_credential_health(&self, db, uuid) -> Result<HealthCheckResult>
```

### 3.2 Kiro Provider 模块

**位置**: `src-tauri/src/providers/kiro.rs`

**核心功能**：
- Kiro OAuth Token 加载和刷新
- 请求格式转换（Anthropic 格式 → CodeWhisperer 格式）
- 响应格式转换（CodeWhisperer 格式 → Anthropic 格式）

**健康检查 URL**：
```
https://codewhisperer.us-east-1.amazonaws.com/SendMessage
```

### 3.3 Flow Monitor 模块

**数据存储位置**：
```
~/Library/Application Support/proxycast/flows/
├── global_index.sqlite      # 索引数据库
└── YYYY-MM-DD/
    └── flows_xxx.jsonl      # 详细请求/响应日志
```

**数据库表结构**：
```sql
CREATE TABLE flow_index (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    status TEXT NOT NULL,        -- Completed / Failed
    duration_ms INTEGER,
    input_tokens INTEGER,
    output_tokens INTEGER,
    has_error INTEGER DEFAULT 0,
    file_path TEXT NOT NULL,     -- 详细日志文件路径
    file_offset INTEGER NOT NULL -- 在日志文件中的偏移量
);
```

---

## 4. Token 刷新机制

### 4.1 自动刷新触发条件

ProxyCast 有两种 Token 刷新方式：

| 触发方式 | 触发条件 | 代码位置 |
|---------|---------|---------|
| **健康检查时刷新** | 检测到 401 错误时自动刷新 | `check_credential_health()` L411-473 |
| **手动刷新** | 用户点击刷新按钮 | `refresh_credential_token()` L1430-1457 |

### 4.2 核心刷新代码

```rust
// 健康检查时如果遇到 401 错误，自动尝试刷新
if e.contains("401") || e.contains("Unauthorized") {
    tracing::info!("检测到 401 错误，尝试刷新 token: {}", uuid);
    
    match self.refresh_credential_token(db, uuid).await {
        Ok(_) => {
            // 刷新成功后重新执行健康检查
            let retry_result = self.perform_health_check(&updated_cred.credential, &check_model).await;
            ...
        }
        Err(refresh_err) => {
            // Token 刷新失败
            ...
        }
    }
}
```

### 4.3 Kiro Token 刷新流程

```rust
// src-tauri/src/services/provider_pool_service.rs L1443-1445
async fn refresh_kiro_token(&self, creds_path: &str) -> Result<String, String> {
    let mut provider = KiroProvider::new();
    provider.load_credentials_from_path(creds_path).await?;
    provider.refresh_token().await
}
```

### 4.4 Token 有效期

Kiro OAuth Token 的典型有效期约为 **30-60 分钟**。

Token 信息存储位置：
```
~/Library/Application Support/proxycast/credentials/kiro_*.json
```

Token 文件结构：
```json
{
  "accessToken": "aoaAAAAAG...",
  "refreshToken": "aorAAAAAG...",
  "expiresAt": "2025-12-29T01:54:06.193Z",
  "lastRefresh": "2025-12-29T01:29:25.375803+00:00",
  "authMethod": "social",
  "profileArn": "arn:aws:codewhisperer:us-east-1:...:profile/..."
}
```

---

## 5. 解决方案与建议

### 5.1 短期解决方案

1. **添加多个凭证**：在凭证池中添加多个 Kiro 账号，实现自动故障转移
2. **定期健康检查**：使用前点击"检测全部"触发自动 Token 刷新
3. **使用其他 Provider**：如果有 Anthropic 官方 API Key，可作为备用

### 5.2 长期改进建议

**建议 1: 添加后台定时 Token 刷新**

```rust
// 建议添加到 ProxyCast 核心服务
async fn background_token_refresh_task(&self, db: &DbConnection) {
    loop {
        // 每 15 分钟检查一次
        tokio::time::sleep(Duration::from_secs(900)).await;
        
        // 检查所有 OAuth 凭证的 Token 有效期
        for cred in self.get_all_oauth_credentials(db) {
            if cred.token_expires_in_minutes() < 10 {
                // 提前刷新即将过期的 Token
                self.refresh_credential_token(db, &cred.uuid).await;
            }
        }
    }
}
```

**建议 2: 改进错误处理**

对于 "error decoding response body" 错误，增加更详细的日志：
- 记录原始响应内容
- 区分是解析错误还是网络超时
- 提供更友好的用户提示

---

## 附录：快速诊断命令

### A. 检查 Kiro Token 有效期

```bash
cat ~/Library/Application\ Support/proxycast/credentials/kiro_*.json | \
  python3 -c "
import json,sys
from datetime import datetime
d = json.load(sys.stdin)
expires = d.get('expiresAt', '')
if expires:
    exp_time = datetime.fromisoformat(expires.replace('Z', '+00:00'))
    now = datetime.now(exp_time.tzinfo)
    remaining = (exp_time - now).total_seconds() / 60
    print(f'过期时间: {expires}')
    print(f'剩余时间: {remaining:.1f} 分钟')
"
```

### B. 查看最近失败的请求

```bash
sqlite3 ~/Library/Application\ Support/proxycast/flows/global_index.sqlite \
  "SELECT id, created_at, model, status, duration_ms FROM flow_index WHERE status='Failed' ORDER BY created_at DESC LIMIT 10;"
```

### C. 测试 ProxyCast API

```bash
curl -s -X POST http://127.0.0.1:8999/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: pc_V2DwytiHa8mbbHxDBQlvXzqnUV3Tdr99" \
  -H "anthropic-version: 2023-06-01" \
  -d '{"model":"claude-opus-4-5","max_tokens":10,"messages":[{"role":"user","content":"Hi"}]}'
```

---

## 更新日志

- **2025-12-29**: 初始版本，记录 Kiro Provider 问题诊断过程
