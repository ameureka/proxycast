# AI Agent 指南

本文件为 AI Agent 在此代码库中工作时提供指导。

## 基本规则

1. **始终使用中文输出** - 所有回复、注释、文档都使用中文

## 构建命令

```bash
# 构建 Tauri 应用
cd src-tauri && cargo build

# 构建前端
npm run build

# 开发模式
npm run tauri dev
```

## 测试命令

```bash
# 运行 Rust 测试
cd src-tauri && cargo test

# 运行前端测试
npm test
```

## 代码检查

```bash
# Rust 代码检查
cd src-tauri && cargo clippy

# 前端代码检查
npm run lint
```

## 项目架构

### 技术栈
- 前端：React + TypeScript + Vite + TailwindCSS
- 后端：Rust + Tauri
- 数据库：SQLite (rusqlite)

### 核心模块

1. **Provider 系统** (`src-tauri/src/providers/`)
   - Kiro/CodeWhisperer OAuth 认证
   - Gemini OAuth 认证
   - Qwen OAuth 认证
   - Antigravity OAuth 认证
   - OpenAI/Claude API Key 认证

2. **凭证池管理** (`src-tauri/src/services/provider_pool_service.rs`)
   - 多凭证轮询负载均衡
   - 健康检查机制
   - Token 自动刷新

3. **API 服务器** (`src-tauri/src/server.rs`)
   - OpenAI 兼容 API 端点
   - Claude 兼容 API 端点
   - 流式响应支持

4. **协议转换** (`src-tauri/src/converter/`)
   - OpenAI ↔ CodeWhisperer 转换
   - OpenAI ↔ Claude 转换

### 凭证管理策略（方案 B）

Kiro 凭证采用完全独立的副本策略：
- 上传凭证时，自动合并 `clientIdHash` 文件中的 `client_id`/`client_secret` 到副本
- 每个副本文件完全独立，支持多账号场景
- 刷新 Token 时只使用副本文件中的凭证，不依赖原始文件

## 开发指南

### 添加新 Provider

1. 在 `src-tauri/src/providers/` 创建新的 provider 模块
2. 实现凭证加载、Token 刷新、API 调用方法
3. 在 `CredentialData` 枚举中添加新类型
4. 在 `ProviderPoolService` 中添加健康检查逻辑

### 修改凭证管理

- 凭证文件存储在 `~/Library/Application Support/proxycast/credentials/`
- 数据库存储凭证元数据和状态
- Token 缓存在数据库中，避免频繁读取文件

### 调试技巧

- 日志输出使用 `tracing` 宏
- API 请求调试文件保存在 `~/.proxycast/logs/`
- 使用 `debug_kiro_credentials` 命令调试凭证加载
