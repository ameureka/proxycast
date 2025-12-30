# ProxyCast 配置注入机制研究

> 研究日期: 2025-12-25  
> 版本: v0.17.11

## 1. 项目概述

**ProxyCast** 是一个 AI API 转换网关的桌面客户端，核心功能是将 Web 端 OAuth 凭证（如 Kiro、Gemini、通义千问）转换为标准 OpenAI/Anthropic API 接口。

### 技术栈
| 层级 | 技术 |
|------|------|
| 前端 | React 18 + Vite + TailwindCSS |
| 后端 | Rust + Tauri v2 + Axum |
| 数据库 | SQLite (rusqlite) |
| 协议 | OpenAI / Anthropic API 兼容 |

---

## 2. 模型配置规范

### 2.1 Kiro 支持的模型 ID

源码位置：`src-tauri/src/converter/openai_to_cw.rs`

| 系列 | 短 ID (推荐) | 带日期版本 | Kiro 内部 ID |
|------|-------------|-----------|--------------|
| **Opus 4.5** | `claude-opus-4-5` | `claude-opus-4-5-20251101` | `claude-opus-4.5` |
| **Haiku 4.5** | `claude-haiku-4-5` | `claude-haiku-4-5-20251001` | `claude-haiku-4.5` |
| **Sonnet 4.5** | `claude-sonnet-4-5` | `claude-sonnet-4-5-20250929` | `CLAUDE_SONNET_4_5_...` |
| **Sonnet 4** | `claude-sonnet-4-20250514` | - | `CLAUDE_SONNET_4_...` |
| **Sonnet 3.7** | `claude-3-7-sonnet-20250219` | - | 旧版兼容 |

### 2.2 模型映射代码

```rust
// src-tauri/src/converter/openai_to_cw.rs
pub fn get_model_map() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    map.insert("claude-opus-4-5", "claude-opus-4.5");
    map.insert("claude-opus-4-5-20251101", "claude-opus-4.5");
    map.insert("claude-haiku-4-5", "claude-haiku-4.5");
    map.insert("claude-sonnet-4-5", "CLAUDE_SONNET_4_5_20250929_V1_0");
    // ...
    map
}
```

---

## 3. 配置注入机制 (Live Sync)

### 3.1 核心原理

ProxyCast 通过**直接修改目标 AI 客户端的配置文件**实现"无感注入"。

源码位置：`src-tauri/src/services/live_sync.rs`

### 3.2 注入流程

```
┌─────────────────────────────────────────────────────────┐
│             ProxyCast "配置管理" UI                      │
│  {                                                      │
│    "env": {                                             │
│      "ANTHROPIC_API_KEY": "pc_zCRMPk...",              │
│      "OPENAI_BASE_URL": "http://127.0.0.1:8999",       │
│    }                                                    │
│  }                                                      │
└───────────────────────┬─────────────────────────────────┘
                        │ sync_claude_settings()
                        ▼
┌─────────────────────────────────────────────────────────┐
│                  ~/.claude.json                         │
│  被注入的环境变量会覆盖 Claude Code 默认行为             │
└───────────────────────┬─────────────────────────────────┘
                        │ Claude Code 启动时读取
                        ▼
┌─────────────────────────────────────────────────────────┐
│              Claude Code / Cursor                       │
│  所有 API 请求被路由到 localhost:8999                   │
└─────────────────────────────────────────────────────────┘
```

### 3.3 目标配置文件

| 客户端 | 配置文件路径 |
|--------|-------------|
| Claude Code | `~/.claude.json` |
| Codex | `~/.codex/auth.json`, `~/.codex/config.toml` |
| Gemini | `~/.gemini/.env`, `~/.gemini/settings.json` |

### 3.4 核心代码解析

```rust
// sync_claude_settings() 简化版
fn sync_claude_settings(provider: &Provider) -> Result<()> {
    let config_path = home.join(".claude.json");
    
    // 1. 读取现有配置
    let mut settings = serde_json::from_str(&content)?;
    
    // 2. 合并 ProxyCast 的 env 变量
    for (key, value) in provider.settings_config.get("env") {
        target_env.insert(key.clone(), value.clone());
    }
    
    // 3. 写回文件
    std::fs::write(&config_path, content)?;
}
```

---

## 4. 凭证池配置

### 4.1 优先级规则
- **数值越小，优先级越高** (1 > 10 > 100)
- **0 表示禁用**
- **同级轮询**：相同 priority 的凭证自动负载均衡

### 4.2 推荐配置策略

```yaml
providers:
  - name: kiro-free-1
    type: kiro
    priority: 1      # 免费账号优先
  - name: kiro-free-2
    type: kiro
    priority: 1      # 同级轮询
  - name: claude-api-backup
    type: claude_custom
    priority: 99     # 付费 API 作为兜底
```

---

## 5. API Server 配置

### 5.1 默认配置

```yaml
server:
  host: "127.0.0.1"
  port: 8999
```

### 5.2 客户端连接示例

**Cursor 配置：**
- API Key: `pc_zCRMPk3BhEgIbsH04kDrDL8PZEbbRzsU`
- Base URL: `http://127.0.0.1:8999/v1`

**Continue (VS Code) 配置：**
```json
{
  "models": [{
    "title": "ProxyCast",
    "provider": "openai",
    "model": "claude-sonnet-4-5",
    "apiBase": "http://127.0.0.1:8999/v1",
    "apiKey": "your-proxycast-api-key"
  }]
}
```

---

## 6. 关键源码位置

| 功能 | 文件路径 |
|------|---------|
| 模型映射 | `src-tauri/src/converter/openai_to_cw.rs` |
| 配置注入 | `src-tauri/src/services/live_sync.rs` |
| 凭证池管理 | `src-tauri/src/services/provider_pool_service.rs` |
| API 路由 | `src-tauri/src/router/amp_router.rs` |
| Provider 实现 | `src-tauri/src/providers/` |

---

## 7. 参考资料

- [ProxyCast 官方文档](https://aiclientproxy.github.io/proxycast/)
- [Provider 概述](https://aiclientproxy.github.io/proxycast/providers/overview)
- [快速开始](https://aiclientproxy.github.io/proxycast/introduction/quickstart)
- [凭证池管理](https://aiclientproxy.github.io/proxycast/user-guide/credential-pool)
- [API Server](https://aiclientproxy.github.io/proxycast/user-guide/api-server)
