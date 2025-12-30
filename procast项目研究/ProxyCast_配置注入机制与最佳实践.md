# ProxyCast 配置注入机制与最佳实践

> **创建日期**: 2025-12-29  
> **适用版本**: ProxyCast + Claude Code v2.0.76

---

## 目录

1. [三个配置文件详解](#1-三个配置文件详解)
2. [环境变量读取优先级](#2-环境变量读取优先级)
3. [ProxyCast 配置注入机制](#3-proxycast-配置注入机制)
4. [两种导入模式对比](#4-两种导入模式对比)
5. [推荐配置方案](#5-推荐配置方案)

---

## 1. 三个配置文件详解

### 1.1 `~/.claude/settings.json` (用户全局设置)

**定位**: Claude Code 的用户级设置文件，所有项目共享。

**示例**:
```json
{
  "env": {
    "ANTHROPIC_API_KEY": "pc_V2DwytiHa8mbbHxDBQlvXzqnUV3Tdr99",
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:8999",
    "ANTHROPIC_MODEL": "claude-opus-4-5",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "claude-sonnet-4-5",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "claude-opus-4-5"
  },
  "model": "opus",
  "enabledPlugins": {
    "claude-mem@thedotmack": true
  }
}
```

---

### 1.2 `~/.claude.json` (状态文件)

**定位**: Claude Code 自动管理的状态文件，存储 OAuth、MCP 配置、项目状态等。

**env 字段示例**:
```json
{
  "ANTHROPIC_AUTH_TOKEN": "pc_V2DwytiHa8mbbHxDBQlvXzqnUV3Tdr99",
  "ANTHROPIC_BASE_URL": "http://127.0.0.1:8999",
  "ANTHROPIC_DEFAULT_OPUS_MODEL": "claude-opus-4-5-20251101",
  "ANTHROPIC_DEFAULT_SONNET_MODEL": "claude-sonnet-4-20250514",
  "ANTHROPIC_MODEL": "claude-opus-4-5-20251101",
  "OPENAI_API_KEY": "pc_V2DwytiHa8mbbHxDBQlvXzqnUV3Tdr99",
  "OPENAI_BASE_URL": "http://127.0.0.1:8999"
}
```

**重要字段**:
- `customApiKeyResponses.rejected`: 被拒绝的 API Key 列表，需保持为空

---

### 1.3 `~/.zshrc` (Shell 环境变量)

**定位**: 系统 Shell 配置，Claude Code 启动时会读取。

**示例**:
```bash
# ======================================================
# ProxyCast 代理配置 - Claude Code 专用
# ======================================================
# 关键：使用 ANTHROPIC_AUTH_TOKEN 而不是 ANTHROPIC_API_KEY
export ANTHROPIC_AUTH_TOKEN="pc_V2DwytiHa8mbbHxDBQlvXzqnUV3Tdr99"
export ANTHROPIC_BASE_URL="http://127.0.0.1:8999"
# 必须设为空，防止 Claude Code 尝试官方认证
export ANTHROPIC_API_KEY=""
# ======================================================
```

---

## 2. 环境变量读取优先级

Claude Code 从以下来源读取环境变量，**优先级从高到低**：

```
1️⃣ ~/.claude.json (env)          ← 最高优先级，会覆盖其他来源
        ⬇
2️⃣ ~/.claude/settings.json (env)
        ⬇
3️⃣ ~/.zshrc (Shell 环境变量)      ← 最低优先级
```

### 关键结论

| 场景 | 建议操作 |
|-----|---------|
| 想快速生效 | 只修改 `~/.claude.json` 的 `env` 字段 |
| 想持久配置 | 通过 ProxyCast "导入配置" 功能自动写入 |
| 配置冲突时 | 检查 `~/.claude.json`，它的优先级最高 |

---

## 3. ProxyCast 配置注入机制

### 3.1 核心原理

ProxyCast 通过 `live_sync.rs` 模块直接修改目标配置文件：

```
┌─────────────────────────────────────────────────────────────┐
│              ProxyCast 配置管理 UI                            │
│                  点击"导入配置"                                │
└───────────────────────────┬─────────────────────────────────┘
                            │ sync_claude_settings()
                            ▼
┌─────────────────────────────────────────────────────────────┐
│               同时写入两个文件                                 │
│                                                              │
│   ① ~/.claude/settings.json                                 │
│      {                                                       │
│        "env": {                                              │
│          "ANTHROPIC_API_KEY": "pc_xxx...",                  │
│          "ANTHROPIC_BASE_URL": "http://127.0.0.1:8999"      │
│        }                                                     │
│      }                                                       │
│                                                              │
│   ② ~/.zshrc (备份)                                          │
│      # >>> ProxyCast Claude Config >>>                       │
│      export ANTHROPIC_AUTH_TOKEN="pc_xxx..."                │
│      export ANTHROPIC_BASE_URL="http://127.0.0.1:8999"      │
│      # <<< ProxyCast Claude Config <<<                       │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│               Claude Code 启动时                              │
│   读取配置 → 所有 API 请求路由到 ProxyCast                     │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 关键源码位置

| 功能 | 文件路径 |
|-----|---------|
| 配置注入主逻辑 | `src-tauri/src/services/live_sync.rs` |
| Claude 配置同步 | `sync_claude_settings()` 函数 |
| Shell 变量写入 | `write_env_to_shell_config()` 函数 |
| 认证冲突清理 | `clean_claude_auth_conflict()` 函数 |
| 配置文件读取 | `read_live_settings()` 函数 |

---

## 4. 两种导入模式对比

### 4.1 本地代理 (ProxyCast)

**工作方式**: 所有请求经过 `http://127.0.0.1:8999` 代理

**写入 Claude Code 的配置**:
```json
{
  "env": {
    "ANTHROPIC_API_KEY": "<ProxyCast API Key>",
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:8999"
  }
}
```

**适用凭证**: Kiro OAuth、Gemini OAuth、通义千问 OAuth、Antigravity OAuth

---

### 4.2 从凭证池导入

**分两种情况**:

#### 情况 A: API Key 类型凭证 (Claude API Key, OpenAI API Key)

**工作方式**: 直连官方 API

**写入 Claude Code 的配置**:
```json
{
  "env": {
    "ANTHROPIC_API_KEY": "<凭证池中的原始 API Key>",
    "ANTHROPIC_BASE_URL": "<凭证池中的 Base URL>"
  }
}
```

#### 情况 B: OAuth 类型凭证 (Kiro, Gemini OAuth)

**工作方式**: 仍需经过 ProxyCast 代理

**写入 Claude Code 的配置**:
```json
{
  "env": {
    "ANTHROPIC_API_KEY": "<ProxyCast API Key>",
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:8999"
  }
}
```

---

### 4.3 对比表

| 特性 | 本地代理 ⭐⭐⭐ | 从凭证池导入 |
|-----|---------------|-------------|
| **配置简洁性** | 只需维护一个 Key | 可能需要管理多个 |
| **负载均衡** | ✅ 自动在多凭证间轮询 | ❌ 单一凭证 |
| **故障转移** | ✅ 失败自动切换 | ❌ 需手动切换 |
| **OAuth 支持** | ✅ 完美支持 | ⚠️ OAuth 仍需走代理 |
| **请求追踪** | ✅ Flow Monitor 可查看 | ❌ 无法追踪 |
| **图标颜色** | 蓝色 `#3b82f6` | 绿色 `#22c55e` |

---

## 5. 推荐配置方案

### ⭐ 推荐：本地代理 (ProxyCast)

**适用场景**:
- 使用 Kiro、Gemini、通义千问等 OAuth 凭证
- 想要负载均衡和自动故障转移
- 需要通过 Flow Monitor 调试请求

**配置步骤**:
1. 在 ProxyCast 凭证池中添加凭证（Kiro、Gemini 等）
2. 进入"配置管理"页面
3. 点击"添加" → 选择"本地代理 ProxyCast"
4. 点击"导入配置"

**最终效果**:
```json
// ~/.claude/settings.json
{
  "env": {
    "ANTHROPIC_API_KEY": "pc_V2DwytiHa8mbbHxDBQlvXzqnUV3Tdr99",
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:8999"
  }
}
```

---

### 备选：从凭证池导入 API Key

**适用场景**:
- 有官方 Anthropic API Key
- 想直连官方 API，不经过代理
- 不需要负载均衡

**配置步骤**:
1. 在凭证池中添加 "Claude API Key" 类型凭证
2. 进入"配置管理"页面
3. 点击"添加" → 选择"从凭证池导入"
4. 选择对应的 API Key 凭证

---

## 附录：快速检查配置一致性

```bash
# 检查 ProxyCast 服务端 API Key
grep "api_key:" ~/Library/Application\ Support/proxycast/config.yaml

# 检查 ~/.claude/settings.json 中的 Key
cat ~/.claude/settings.json | python3 -c "import json,sys; print(json.load(sys.stdin).get('env',{}).get('ANTHROPIC_API_KEY','未设置'))"

# 检查 ~/.claude.json 中的 Key
cat ~/.claude.json | python3 -c "import json,sys; print(json.load(sys.stdin).get('env',{}).get('ANTHROPIC_AUTH_TOKEN','未设置'))"

# 检查 Shell 环境变量
echo $ANTHROPIC_AUTH_TOKEN
```

**这四个值必须一致**（都应该是 ProxyCast 服务端的 API Key）！

---

## 更新日志

- **2025-12-29**: 初始版本，整理配置文件、优先级、注入机制和最佳实践
