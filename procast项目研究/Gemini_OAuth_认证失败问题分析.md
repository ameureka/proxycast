# Gemini OAuth 认证问题完整分析报告

> **创建日期**: 2025-12-25  
> **问题状态**: Token 刷新已修复 ✅ | API 路由待实现 ⚠️

---

## 目录

1. [问题现象](#1-问题现象)
2. [根因分析](#2-根因分析)
3. [本地凭证分析](#3-本地凭证分析)
4. [解决方案与验证](#4-解决方案与验证)
5. [功能缺失分析](#5-功能缺失分析)
6. [ProxyCast 与 Gemini 配置关系](#6-proxycast-与-gemini-配置关系)
7. [快速检查脚本](#7-快速检查脚本)

---

## 1. 问题现象

ProxyCast 凭证池中的 Gemini 凭证显示：

```
刷新 Token 失败: Token 已过期，正在尝试刷新。
[GEMINI] Token 刷新失败。HTTP 400 - { 
  "error": "invalid_request", 
  "error_description": "Could not determine client ID from request."
}
```

---

## 2. 根因分析

### 2.1 问题代码路径

```
凭证池健康检查
    ↓
TokenCacheService.refresh_gemini()
    ↓
GeminiProvider.refresh_token()
    ↓
get_oauth_client_id() / get_oauth_client_secret()
    ↓
std::env::var("GEMINI_OAUTH_CLIENT_ID")  ← 返回空字符串！
```

### 2.2 关键代码

**文件**: `src-tauri/src/providers/gemini.rs`

```rust
// 第 26-32 行：从环境变量读取（当前实现）
fn get_oauth_client_id() -> String {
    std::env::var("GEMINI_OAUTH_CLIENT_ID").unwrap_or_default()  // ❌ 返回空字符串
}

fn get_oauth_client_secret() -> String {
    std::env::var("GEMINI_OAUTH_CLIENT_SECRET").unwrap_or_default()  // ❌ 返回空字符串
}

// 第 762-764 行：硬编码的常量（未被刷新函数使用！）
pub const GEMINI_OAUTH_CLIENT_ID: &str =
    "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com";
pub const GEMINI_OAUTH_CLIENT_SECRET: &str = "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";
```

---

## 3. 本地凭证分析

### 3.1 凭证文件位置

| 文件 | 路径 |
|------|------|
| OAuth 凭证 | `~/.gemini/oauth_creds.json` |
| 设置 | `~/.gemini/settings.json` |
| Google 账户 | `~/.gemini/google_accounts.json` |

### 3.2 凭证文件内容

```json
{
  "access_token": "ya29.a0...",
  "scope": "https://www.googleapis.com/auth/userinfo.email openid ...",
  "token_type": "Bearer",
  "expiry_date": 1766609714856,
  "refresh_token": "1//0gD1Q0YxDL3Yu..."
}
```

> ⚠️ **关键发现**: 凭证文件中**没有** `client_id` 和 `client_secret`  
> Gemini CLI 在登录时不会将这些值保存到凭证文件

---

## 4. 解决方案与验证

### 4.1 解决方案：设置环境变量

在 `~/.zshrc` 中添加：

```bash
# Gemini OAuth 配置 - ProxyCast Token 刷新必需
export GEMINI_OAUTH_CLIENT_ID="681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com"
export GEMINI_OAUTH_CLIENT_SECRET="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"
```

### 4.2 验证结果

| 测试项 | 结果 | 说明 |
|--------|------|------|
| Token 刷新 (curl 手动测试) | ✅ 成功 | 返回新的 access_token |
| 凭证池健康检查 | ✅ 通过 | UI 显示绿色"健康检查通过!" |
| /v1/models 端点 | ✅ 可用 | 列出 5 个 Gemini 模型 |

**Token 刷新测试输出**:
```
✅ Token 刷新成功!
新 access_token: ya29.a0Aa7pCA-xT2VsBmkuaeyY4LX...
过期时间: 3599 秒
```

---

## 5. 功能缺失分析

### 5.1 Gemini OAuth 路由未实现

虽然 Token 刷新已修复，但 **ProxyCast 尚未实现 Gemini OAuth 的 API 调用路由**。

**测试结果**:
```bash
curl http://127.0.0.1:8999/v1/chat/completions \
  -H "Authorization: Bearer pc_xxx" \
  -d '{"model": "gemini-2.5-flash", ...}'

# 返回:
{"error": {"message": "Gemini OAuth routing not yet implemented."}}
```

### 5.2 代码位置

**文件**: `src-tauri/src/server/handlers/provider_calls.rs`

```rust
// 第 281-288 行 (Anthropic 格式)
CredentialData::GeminiOAuth { .. } => {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({"error": {"message": "Gemini OAuth routing not yet implemented."}})),
    ).into_response()
}

// 第 814-816 行 (OpenAI 格式)
CredentialData::GeminiOAuth { .. } => {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({"error": {"message": "Gemini OAuth routing not yet implemented."}})),
    ).into_response()
}
```

### 5.3 支持状态对比

| Provider 类型 | Anthropic 格式 | OpenAI 格式 |
|---------------|----------------|-------------|
| Kiro OAuth | ✅ 支持 | ✅ 支持 |
| Antigravity OAuth | ✅ 支持 | ✅ 支持 |
| OpenAI Key | ✅ 支持 | ✅ 支持 |
| Claude Key | ✅ 支持 | ✅ 支持 |
| **Gemini OAuth** | ❌ 未实现 | ❌ 未实现 |

---

## 6. ProxyCast 与 Gemini 配置关系

### 6.1 架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                     Gemini CLI 登录流程                          │
├─────────────────────────────────────────────────────────────────┤
│  1. 用户运行 gemini 命令触发 OAuth 登录                          │
│  2. 浏览器打开 Google 授权页面                                   │
│  3. 用户授权后，CLI 获取 authorization code                     │
│  4. CLI 使用内置的 CLIENT_ID/SECRET 换取 access/refresh token   │
│  5. Token 保存到 ~/.gemini/oauth_creds.json                     │
│  6. ❌ CLIENT_ID/SECRET 不会保存到文件                           │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                    ProxyCast Token 刷新                         │
├─────────────────────────────────────────────────────────────────┤
│  1. 读取 ~/.gemini/oauth_creds.json 的 refresh_token            │
│  2. 调用 get_oauth_client_id() 获取 CLIENT_ID                   │
│  3. ❌ (修复前) 环境变量未设置，返回空字符串                       │
│  4. ✅ (修复后) 从环境变量读取正确的 CLIENT_ID/SECRET            │
│  5. ✅ Google OAuth 返回新的 access_token                       │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                    ProxyCast API 调用 (待实现)                   │
├─────────────────────────────────────────────────────────────────┤
│  1. 客户端请求 /v1/chat/completions (model=gemini-2.5-flash)    │
│  2. ProxyCast 选择 Gemini OAuth 凭证                            │
│  3. ❌ 当前返回 "Gemini OAuth routing not yet implemented."     │
│  4. ⚠️ 需要开发 GeminiOAuth -> Gemini API 的转换和调用逻辑      │
└─────────────────────────────────────────────────────────────────┘
```

### 6.2 相关文件列表

| 文件 | 作用 |
|------|------|
| `src-tauri/src/providers/gemini.rs` | Gemini Provider 实现、Token 刷新 |
| `src-tauri/src/server/handlers/provider_calls.rs` | API 调用路由（待实现） |
| `src-tauri/src/services/provider_pool_service.rs` | 凭证池服务 |
| `src-tauri/src/services/token_cache_service.rs` | Token 缓存服务 |
| `~/.gemini/oauth_creds.json` | 本地 OAuth 凭证 |

---

## 7. 快速检查脚本

### 7.1 一键诊断

```bash
#!/bin/bash
echo "=== Gemini 配置诊断 ==="

# 1. 检查环境变量
echo ""
echo "1️⃣  环境变量:"
echo "   GEMINI_OAUTH_CLIENT_ID=${GEMINI_OAUTH_CLIENT_ID:0:20}..."
echo "   GEMINI_OAUTH_CLIENT_SECRET=${GEMINI_OAUTH_CLIENT_SECRET:0:10}..."

# 2. 检查凭证文件
echo ""
echo "2️⃣  凭证文件:"
if [ -f ~/.gemini/oauth_creds.json ]; then
    echo "   ✅ ~/.gemini/oauth_creds.json 存在"
    REFRESH=$(cat ~/.gemini/oauth_creds.json | python3 -c "import json,sys; print(json.load(sys.stdin).get('refresh_token','')[:20])")
    echo "   refresh_token: ${REFRESH}..."
else
    echo "   ❌ ~/.gemini/oauth_creds.json 不存在"
fi

# 3. 测试 Token 刷新
echo ""
echo "3️⃣  Token 刷新测试:"
if [ -n "$GEMINI_OAUTH_CLIENT_ID" ]; then
    RESULT=$(curl -s -X POST https://oauth2.googleapis.com/token \
      -d "client_id=$GEMINI_OAUTH_CLIENT_ID" \
      -d "client_secret=$GEMINI_OAUTH_CLIENT_SECRET" \
      -d "refresh_token=$(cat ~/.gemini/oauth_creds.json | python3 -c 'import json,sys; print(json.load(sys.stdin)["refresh_token"])')" \
      -d "grant_type=refresh_token")
    if echo "$RESULT" | grep -q "access_token"; then
        echo "   ✅ Token 刷新成功"
    else
        echo "   ❌ Token 刷新失败: $RESULT"
    fi
else
    echo "   ⚠️  跳过 (环境变量未设置)"
fi

# 4. 检查 ProxyCast
echo ""
echo "4️⃣  ProxyCast 状态:"
if curl -s http://127.0.0.1:8999/health | grep -q "healthy"; then
    echo "   ✅ ProxyCast 运行中"
    GEMINI_MODELS=$(curl -s http://127.0.0.1:8999/v1/models | python3 -c "import json,sys; print(len([m for m in json.load(sys.stdin).get('data',[]) if 'gemini' in m['id']]))")
    echo "   Gemini 模型数: $GEMINI_MODELS"
else
    echo "   ❌ ProxyCast 未运行"
fi
```

---

## 总结

| 问题 | 状态 | 解决方案 |
|------|------|---------|
| Token 刷新失败 | ✅ 已修复 | 设置 `GEMINI_OAUTH_CLIENT_ID` 和 `GEMINI_OAUTH_CLIENT_SECRET` 环境变量 |
| 凭证池健康检查 | ✅ 通过 | 环境变量设置后自动恢复 |
| Gemini API 调用 | ⚠️ 待实现 | 需要开发 `CredentialData::GeminiOAuth` 的路由逻辑 |

---

## 下一步

如需使用 Gemini OAuth 凭证进行 API 调用，需要实现：

1. `call_provider_anthropic` 中的 `GeminiOAuth` 分支
2. `call_provider_openai` 中的 `GeminiOAuth` 分支
3. 请求转换：OpenAI/Anthropic 格式 → Gemini Native 格式
4. 响应转换：Gemini Native 响应 → OpenAI/Anthropic 格式
