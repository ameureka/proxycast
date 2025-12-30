---
description: 如何调试 ProxyCast 凭证和认证问题
---

# 凭证调试工作流

## 常见问题

1. Token 刷新失败
2. 凭证加载失败
3. API 请求 401/403 错误
4. Provider 显示为不健康

## 步骤 1：检查日志

```bash
// turbo
cat ~/.proxycast/logs/$(ls -t ~/.proxycast/logs/ | head -1)
```

## 步骤 2：查看凭证文件

Kiro 凭证位置：
```bash
// turbo
ls -la ~/Library/Application\ Support/proxycast/credentials/
```

## 步骤 3：使用调试命令

在开发模式下可以使用 Tauri 命令调试：
```javascript
// 在前端 DevTools 中
invoke('debug_kiro_credentials')
```

## 步骤 4：检查数据库

```bash
// turbo
sqlite3 ~/Library/Application\ Support/proxycast/proxycast.db "SELECT * FROM credentials;"
```

## 步骤 5：手动测试 Token

```bash
# 使用凭证中的 access_token 测试 API
curl -X POST "https://api.target-service.com/v1/chat" \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -d '{"test": true}'
```

## 关键代码位置

- Token 刷新：`src-tauri/src/services/token_cache_service.rs`
- 健康检查：`src-tauri/src/services/provider_pool_service.rs`
- Provider 认证：`src-tauri/src/providers/` 各 Provider 文件

## 常见解决方案

### Token 过期
- 检查 `expires_at` 字段
- 手动触发刷新：UI 上点击"刷新凭证"

### 凭证加载失败
- 验证 JSON 格式是否正确
- 检查必填字段是否存在

### 401 错误
- 检查 API Key 格式
- 验证 Base URL 配置
