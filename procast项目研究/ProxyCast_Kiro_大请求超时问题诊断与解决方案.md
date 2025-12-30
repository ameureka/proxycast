# ProxyCast + Kiro 大请求超时问题诊断与解决方案

> **创建日期**: 2025-12-30  
> **问题类型**: Claude Code 通过 ProxyCast 使用 Kiro 时大请求超时  
> **涉及版本**: ProxyCast 0.8.x / Claude Code v2.0.76+

---

## 目录

1. [问题现象](#1-问题现象)
2. [诊断过程](#2-诊断过程)
3. [根因分析](#3-根因分析)
4. [Claude Code 使用技巧](#4-claude-code-使用技巧)
5. [ProxyCast 配置建议](#5-proxycast-配置建议)

---

## 1. 问题现象

### 1.1 错误表现

| 位置 | 错误信息 |
|-----|---------|
| Claude Code | `API Error: 500 {"error":{"message":"error decoding response body"}}` |
| 凭证池 | Kiro 凭证显示多次错误 |
| Flow Monitor | 请求状态 `Failed`，错误类型 `server_error` / `network` |

### 1.2 失败请求特征

| 特征 | 值 |
|-----|---|
| 模型 | `claude-opus-4-5` |
| 耗时 | ~65-68 秒 (超时) |
| TTFB | `-` (无首字节返回) |
| 响应内容 | 空 ("暂无响应数据") |

---

## 2. 诊断过程

### 2.1 初步检查

1. **检查 Kiro Token 有效期**
   ```bash
   cat ~/Library/Application\ Support/proxycast/credentials/kiro_*.json | \
     python3 -c "
   import json,sys
   from datetime import datetime, timezone
   d = json.load(sys.stdin)
   expires = d.get('expiresAt', '')
   if expires:
       exp_time = datetime.fromisoformat(expires.replace('Z', '+00:00'))
       now = datetime.now(timezone.utc)
       remaining = (exp_time - now).total_seconds() / 60
       print(f'过期时间: {expires}')
       print(f'剩余时间: {remaining:.1f} 分钟')
   "
   ```

2. **查看最近失败请求**
   ```bash
   sqlite3 ~/Library/Application\ Support/proxycast/flows/global_index.sqlite \
     "SELECT id, created_at, model, status, duration_ms FROM flow_index 
      WHERE status='Failed' ORDER BY created_at DESC LIMIT 5;"
   ```

### 2.2 直接 API 测试

测试 Kiro API 是否正常响应：

```bash
# 获取最新 Token
TOKEN_FILE=$(ls -t ~/Library/Application\ Support/proxycast/credentials/kiro_*.json | head -1)
ACCESS_TOKEN=$(cat "$TOKEN_FILE" | python3 -c "import json,sys; print(json.load(sys.stdin).get('accessToken',''))")

# 测试简单请求
curl -s -X POST \
  "https://codewhisperer.us-east-1.amazonaws.com/generateAssistantResponse" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -d '{
    "conversationState": {
      "chatTriggerType": "MANUAL", 
      "currentMessage": {
        "userInputMessage": {"content": "Hi"}
      }
    }
  }' -w "\n状态码: %{http_code}\n"
```

### 2.3 测试结果对比

| 测试类型 | 结果 | 说明 |
|---------|------|------|
| 简单请求 `"Hi"` | ✅ 200 OK | Kiro API 正常响应 |
| Claude Code 完整请求 | ❌ 超时失败 | 包含系统提示+工具定义 |

---

## 3. 根因分析

### 3.1 核心问题

**Kiro Free 账户处理大请求时超时**

Claude Code 发送的请求体通常包含：
- 系统提示词 (System Prompt)
- 工具定义 (Tools Definition)
- 历史上下文 (Context)
- 用户消息 (User Message)

这些内容加起来可能达到 **数万 tokens**，Kiro Free 账户处理这类请求时：
1. 处理时间过长
2. 在返回首字节前就超时
3. ProxyCast 无法解析空响应，报错 `error decoding response body`

### 3.2 为什么简单请求成功

简单请求（如 "Hi"）请求体很小，Kiro 可以快速处理并返回。

### 3.3 错误传播链

```
Claude Code 发送大请求
    ↓
ProxyCast 转发到 Kiro
    ↓
Kiro 开始处理，但处理时间过长
    ↓
~65秒后 ProxyCast 超时
    ↓
ProxyCast 尝试解析空响应 → 失败
    ↓
返回 error decoding response body
```

---

## 4. Claude Code 使用技巧

### 4.1 定期清理上下文

```bash
# 在 Claude Code 中输入
/clear
```

**作用**: 清除当前会话的历史上下文，大幅减少请求体大小

**建议**: 每完成一个独立任务后执行 `/clear`

### 4.2 分段式提问

❌ **错误做法**:
```
帮我分析这个项目的所有代码，然后重构整个认证模块，
同时优化数据库查询，最后添加单元测试
```

✅ **正确做法**:
```
第1步: 帮我分析认证模块的现有实现
(等待完成)

第2步: 基于分析，重构认证模块
(等待完成)

第3步: 添加单元测试
```

### 4.3 减少同时打开的文件

Claude Code 会将打开的文件作为上下文发送：
- ✅ 只保留当前任务需要的文件
- ❌ 避免同时打开大量无关文件

### 4.4 精简系统提示词

如果你有自定义的 `CLAUDE.md` 或 System Prompt：
- 保持精简，避免冗长的指令
- 使用简洁的规则描述
- 删除不常用的规则

### 4.5 使用 Compact 模式

```bash
# 在 Claude Code 中
/compact
```

**作用**: 压缩历史消息，减少 token 消耗

### 4.6 针对 Kiro 的特殊建议

| 场景 | 建议 |
|-----|------|
| **新会话** | 从简单问题开始，逐步加深 |
| **复杂任务** | 拆分成多个小任务 |
| **超时后** | 立即 `/clear` 重新开始 |
| **大文件** | 使用 `@file` 引用而不是粘贴内容 |
| **长对话** | 定期使用 `/compact` 压缩历史 |

### 4.7 快速命令速查

| 命令 | 作用 |
|-----|------|
| `/clear` | 清除上下文 |
| `/compact` | 压缩历史 |
| `/cost` | 查看 token 消耗 |
| `/model` | 切换模型 |
| `/help` | 查看所有命令 |

---

## 5. ProxyCast 配置建议

### 5.1 增加超时时间

如果你有 ProxyCast 的代码访问权限，可以考虑增加请求超时时间。

当前超时约 65 秒，可以尝试增加到 120 秒。

### 5.2 使用多凭证轮换

在凭证池中添加多个 Kiro 账号：
- 实现自动故障转移
- 分散请求负载

### 5.3 考虑使用付费账户

Kiro Free 账户限制：
- 处理能力有限
- 大请求容易超时

付费账户可能有更好的处理能力和更高的超时限制。

### 5.4 备用提供商

如果 Kiro 持续不稳定，可以配置备用提供商：
- Gemini OAuth
- Claude 官方 API Key

---

## 附录：诊断命令汇总

```bash
# 检查 Kiro Token 有效期
cat ~/Library/Application\ Support/proxycast/credentials/kiro_*.json | \
  python3 -c "import json,sys; from datetime import datetime,timezone; d=json.load(sys.stdin); \
  exp=datetime.fromisoformat(d.get('expiresAt','').replace('Z','+00:00')); \
  print(f'剩余: {(exp-datetime.now(timezone.utc)).total_seconds()/60:.1f}分钟')"

# 查看失败请求
sqlite3 ~/Library/Application\ Support/proxycast/flows/global_index.sqlite \
  "SELECT * FROM flow_index WHERE status='Failed' ORDER BY created_at DESC LIMIT 5;"

# 测试 ProxyCast 连通性
curl -s http://127.0.0.1:8999/healthz

# 测试简单 API 调用
curl -s -X POST http://127.0.0.1:8999/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: pc_V2DwytiHa8mbbHxDBQlvXzqnUV3Tdr99" \
  -H "anthropic-version: 2023-06-01" \
  -d '{"model":"claude-sonnet-4-5","max_tokens":10,"messages":[{"role":"user","content":"Hi"}]}'
```

---

## 更新日志

- **2025-12-30**: 初始版本
  - 记录大请求超时问题诊断过程
  - 添加 Claude Code 使用技巧
  - 添加 ProxyCast 配置建议
