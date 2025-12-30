---
description: 帮助大模型快速理解 ProxyCast 项目架构和代码组织
---

# ProxyCast 项目认知指南

## 1. 项目定位

ProxyCast 是一个 **AI API 转换网关桌面客户端**，核心功能：
- 将多个 AI 服务（Kiro、Gemini、通义千问、Antigravity 等）的 OAuth 凭证统一管理
- 转换为标准 OpenAI/Anthropic API 格式，供其他工具调用

## 2. 技术栈一览

| 层级 | 技术 | 关键文件 |
|------|------|----------|
| **前端** | React 18 + Vite + TailwindCSS | `src/` |
| **后端** | Rust + Tauri v2 + Axum | `src-tauri/src/` |
| **数据库** | SQLite (rusqlite) | `src-tauri/src/database/` |
| **配置** | YAML + JSON | `src-tauri/src/config/` |

## 3. 核心目录结构

```
proxycast/
├── src/                          # React 前端
│   ├── components/               # UI 组件 (shadcn/ui)
│   ├── pages/                    # 页面路由
│   ├── hooks/                    # React Hooks
│   └── lib/                      # 工具库
│
├── src-tauri/src/                # Rust 后端
│   ├── providers/                # ⭐ Provider 认证实现
│   │   ├── kiro.rs               # Kiro/CodeWhisperer OAuth
│   │   ├── gemini.rs             # Gemini OAuth
│   │   ├── qwen.rs               # 通义千问 OAuth
│   │   ├── antigravity.rs        # Antigravity OAuth
│   │   ├── claude_oauth.rs       # Claude OAuth
│   │   ├── claude_custom.rs      # Claude API Key
│   │   ├── openai_custom.rs      # OpenAI API Key
│   │   └── vertex.rs             # Vertex AI
│   │
│   ├── services/                 # ⭐ 业务服务层
│   │   ├── provider_pool_service.rs  # 凭证池管理（负载均衡、健康检查）
│   │   ├── token_cache_service.rs    # Token 缓存和刷新
│   │   ├── live_sync.rs              # 配置注入到客户端
│   │   └── usage_service.rs          # 用量统计
│   │
│   ├── converter/                # ⭐ 协议转换
│   │   └── openai_to_cw.rs       # OpenAI ↔ CodeWhisperer
│   │
│   ├── router/                   # API 路由
│   │   └── amp_router.rs         # 主路由配置
│   │
│   ├── server/                   # HTTP 服务器
│   ├── commands/                 # Tauri 命令（前端调用入口）
│   ├── database/                 # 数据库层
│   ├── credential/               # 凭证池抽象
│   └── resilience/               # 弹性策略（重试、故障转移）
```

## 4. 关键概念

### 4.1 Provider
每种 AI 服务对应一个 Provider 实现，负责：
- OAuth 认证流程
- Token 刷新
- API 请求封装

### 4.2 凭证池 (Credential Pool)
- 支持同一 Provider 类型的多个凭证
- 按优先级轮询负载均衡
- 健康检查和自动禁用

### 4.3 配置注入 (Live Sync)
- 自动将 API 配置注入到 `~/.claude.json` 等目标配置文件
- 让 Claude Code、Cursor 等工具透明使用 ProxyCast 代理

### 4.4 协议转换
- 将 OpenAI API 请求转换为目标 Provider 的原生格式
- 支持流式响应

## 5. 开发常用命令

```bash
// turbo-all
# 开发模式
npm run tauri dev

# 构建 Rust 后端
cd src-tauri && cargo build

# 运行 Rust 测试
cd src-tauri && cargo test

# 代码检查
cd src-tauri && cargo clippy
npm run lint
```

## 6. 理解代码的建议路径

1. **从入口开始**：`src-tauri/src/lib.rs` 了解模块注册
2. **看 Provider**：`src-tauri/src/providers/kiro.rs` 了解典型认证流程
3. **看服务层**：`src-tauri/src/services/provider_pool_service.rs` 了解凭证管理
4. **看 API 路由**：`src-tauri/src/router/amp_router.rs` 了解请求处理

## 7. 常见开发任务

### 添加新 Provider
1. 在 `src-tauri/src/providers/` 创建新模块
2. 实现 `Provider` trait
3. 在 `CredentialData` 枚举添加新类型
4. 在 `ProviderPoolService` 添加健康检查

### 修改 API 兼容性
- 查看 `src-tauri/src/converter/` 目录
- 修改模型映射表

### 调试凭证问题
- 使用 `debug_kiro_credentials` 命令
- 查看 `~/.proxycast/logs/` 日志
