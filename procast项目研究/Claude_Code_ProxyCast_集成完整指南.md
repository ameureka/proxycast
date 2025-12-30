# Claude Code + ProxyCast 集成完整指南

> 本文档总结了 Claude Code v2.0.76 与 ProxyCast 集成的完整技术方案，包含原理分析、配置要求、常用命令和故障排除脚本。

---

## 目录

1. [架构原理](#1-架构原理)
2. [认证机制解析](#2-认证机制解析)
3. [配置文件详解](#3-配置文件详解)
4. [环境变量配置](#4-环境变量配置)
5. [快速检查脚本](#5-快速检查脚本)
6. [常见问题排查](#6-常见问题排查)
7. [一键修复脚本](#7-一键修复脚本)

---

## 1. 架构原理

### 1.1 ProxyCast 工作流程

```
┌─────────────┐      ┌──────────────┐      ┌───────────────┐
│ Claude Code │ ──▶  │  ProxyCast   │ ──▶  │  Kiro / AWS   │
│   (客户端)   │      │  (代理服务器) │      │   (后端API)   │
└─────────────┘      └──────────────┘      └───────────────┘
       │                    │
       │  x-api-key         │  Kiro Token
       │  (ProxyCast Key)   │  (自动刷新)
       ▼                    ▼
```

### 1.2 关键组件

| 组件 | 路径 | 作用 |
|------|------|------|
| ProxyCast 服务 | `http://127.0.0.1:8999` | 本地代理服务器 |
| ProxyCast 配置 | `~/Library/Application Support/proxycast/config.yaml` | 服务端 API Key |
| Claude 全局配置 | `~/.claude.json` | Claude Code 环境变量 |
| Claude 设置 | `~/.claude/settings.json` | 模型配置 |
| Shell 配置 | `~/.zshrc` | 环境变量注入 |

### 1.3 Live Sync 机制

ProxyCast 通过 `live_sync.rs` 模块自动将配置注入到客户端：

```rust
// 代码位置: src-tauri/src/services/live_sync.rs
fn sync_claude_settings(provider: &Provider) {
    // 读取 ~/.claude.json
    // 注入 env.ANTHROPIC_AUTH_TOKEN, env.ANTHROPIC_BASE_URL
    // 清理冲突的认证变量
    // 写回配置文件
}
```

---

## 2. 认证机制解析

### 2.1 Claude Code 认证优先级

Claude Code v2.0.76 使用以下优先级读取认证信息：

```
1. ~/.claude.json 中的 env 配置 (最高优先级)
2. 环境变量 ANTHROPIC_AUTH_TOKEN
3. 环境变量 ANTHROPIC_API_KEY
4. macOS Keychain (官方OAuth)
```

### 2.2 第三方代理认证 (关键!)

> **重要**: 使用第三方代理时，必须使用 `ANTHROPIC_AUTH_TOKEN` 而不是 `ANTHROPIC_API_KEY`

| 场景 | 使用的变量 | 说明 |
|------|-----------|------|
| 官方 Anthropic API | `ANTHROPIC_API_KEY` | 直连官方服务 |
| **第三方代理 (ProxyCast)** | `ANTHROPIC_AUTH_TOKEN` | 绕过官方认证 |

### 2.3 ProxyCast API Key 验证逻辑

```rust
// 代码位置: src-tauri/src/server/handlers/api.rs
pub async fn verify_api_key_anthropic(headers, expected_key) {
    // 从 x-api-key 或 authorization 头读取
    // 与 config.yaml 中的 server.api_key 比较
    // 不匹配则返回 401 Invalid API key
}
```

---

## 3. 配置文件详解

### 3.1 ProxyCast 服务端配置

**文件**: `~/Library/Application Support/proxycast/config.yaml`

```yaml
server:
  host: 127.0.0.1
  port: 8999
  api_key: pc_zCRMPk3BhEgIbsH04kDrDL8PZEbbRzsU  # 客户端必须匹配此Key
  tls:
    enable: false

providers:
  kiro:
    enabled: true
    credentials_path: ~/.aws/sso/cache/kiro-auth-token.json
    region: us-east-1

default_provider: kiro
```

### 3.2 Claude 全局配置

**文件**: `~/.claude.json`

```json
{
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "pc_zCRMPk3BhEgIbsH04kDrDL8PZEbbRzsU",
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:8999",
    "ANTHROPIC_API_KEY": ""
  },
  "customApiKeyResponses": {
    "rejected": [],
    "approved": []
  }
}
```

> **关键配置说明**:
> - `ANTHROPIC_AUTH_TOKEN`: 填入 ProxyCast 的 API Key (必须与服务端一致)
> - `ANTHROPIC_BASE_URL`: 指向本地 ProxyCast 服务
> - `ANTHROPIC_API_KEY`: 必须设为**空字符串**，否则会与 AUTH_TOKEN 冲突
> - `customApiKeyResponses.rejected`: 必须为空数组，否则 Key 会被拒绝

### 3.3 Claude 设置文件

**文件**: `~/.claude/settings.json`

```json
{
  "env": {
    "ANTHROPIC_API_KEY": "pc_zCRMPk3BhEgIbsH04kDrDL8PZEbbRzsU",
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:8999"
  },
  "permissions": {
    "allow": [
      "Bash(*)",
      "WebFetch(*)"
    ]
  }
}
```

---

## 4. 环境变量配置

### 4.1 ~/.zshrc 配置

在 `~/.zshrc` 末尾添加：

```bash
# ======================================================
# ProxyCast 代理配置 - Claude Code 专用
# ======================================================
# 关键：使用 ANTHROPIC_AUTH_TOKEN 而不是 ANTHROPIC_API_KEY
export ANTHROPIC_AUTH_TOKEN="pc_zCRMPk3BhEgIbsH04kDrDL8PZEbbRzsU"
export ANTHROPIC_BASE_URL="http://127.0.0.1:8999"
# 必须设为空，防止 Claude Code 尝试官方认证
export ANTHROPIC_API_KEY=""
# ======================================================
```

### 4.2 应用配置

```bash
source ~/.zshrc
```

---

## 5. 快速检查脚本

### 5.1 一键诊断脚本

```bash
#!/bin/bash
# 保存为: ~/check_claude_proxycast.sh

echo "======================================"
echo "Claude Code + ProxyCast 配置诊断"
echo "======================================"

# 1. 检查 ProxyCast 服务状态
echo ""
echo "1️⃣ ProxyCast 服务状态:"
if curl -s http://127.0.0.1:8999/health | grep -q "healthy"; then
    echo "   ✅ ProxyCast 运行正常"
    curl -s http://127.0.0.1:8999/health
else
    echo "   ❌ ProxyCast 未运行或不可达"
fi

# 2. 检查环境变量
echo ""
echo "2️⃣ 环境变量:"
echo "   ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY:-[未设置]}"
echo "   ANTHROPIC_BASE_URL=${ANTHROPIC_BASE_URL:-[未设置]}"
echo "   ANTHROPIC_AUTH_TOKEN=${ANTHROPIC_AUTH_TOKEN:0:20}...${ANTHROPIC_AUTH_TOKEN: -10}"

# 3. 检查 ~/.claude.json
echo ""
echo "3️⃣ ~/.claude.json env 配置:"
if [ -f ~/.claude.json ]; then
    python3 -c "import json; d=json.load(open('$HOME/.claude.json')); print('   AUTH_TOKEN:', d.get('env',{}).get('ANTHROPIC_AUTH_TOKEN','[未设置]')[:20]+'...'); print('   BASE_URL:', d.get('env',{}).get('ANTHROPIC_BASE_URL','[未设置]')); print('   API_KEY:', repr(d.get('env',{}).get('ANTHROPIC_API_KEY','[未设置]')))"
else
    echo "   ❌ 文件不存在"
fi

# 4. 检查 rejected 列表
echo ""
echo "4️⃣ rejected 列表:"
if [ -f ~/.claude.json ]; then
    REJECTED=$(python3 -c "import json; d=json.load(open('$HOME/.claude.json')); print(d.get('customApiKeyResponses',{}).get('rejected',[]))")
    if [ "$REJECTED" = "[]" ]; then
        echo "   ✅ 无被拒绝的 Key"
    else
        echo "   ⚠️ 存在被拒绝的 Key: $REJECTED"
    fi
fi

# 5. 测试 API 连通性
echo ""
echo "5️⃣ API 连通性测试:"
RESPONSE=$(curl -s http://127.0.0.1:8999/v1/messages \
  -H "x-api-key: ${ANTHROPIC_AUTH_TOKEN}" \
  -H "content-type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{"model": "claude-sonnet-4-5", "max_tokens": 5, "messages": [{"role": "user", "content": "Hi"}]}' 2>&1)

if echo "$RESPONSE" | grep -q '"content"'; then
    echo "   ✅ API 调用成功"
elif echo "$RESPONSE" | grep -q "Invalid API key"; then
    echo "   ❌ API Key 不匹配"
    echo "   提示: 检查环境变量与 ProxyCast config.yaml 中的 api_key 是否一致"
else
    echo "   ⚠️ 其他错误: $RESPONSE"
fi

echo ""
echo "======================================"
```

### 5.2 使用方法

```bash
chmod +x ~/check_claude_proxycast.sh
~/check_claude_proxycast.sh
```

---

## 6. 常见问题排查

### 6.1 错误: Invalid API key (401)

**原因**: 客户端发送的 API Key 与 ProxyCast 配置不匹配

**排查步骤**:

```bash
# 1. 查看 ProxyCast 配置中的 API Key
cat ~/Library/Application\ Support/proxycast/config.yaml | grep api_key

# 2. 查看环境变量中的 API Key
echo $ANTHROPIC_AUTH_TOKEN

# 3. 查看 ~/.claude.json 中的 API Key
cat ~/.claude.json | python3 -c "import json,sys; print(json.load(sys.stdin).get('env',{}).get('ANTHROPIC_AUTH_TOKEN'))"
```

**解决**: 确保三处的 API Key 完全一致（注意区分相似字符如 `Z` 和 `2`）

### 6.2 错误: Key 被 rejected

**排查**:

```bash
cat ~/.claude.json | python3 -c "import json,sys; print(json.load(sys.stdin).get('customApiKeyResponses',{}).get('rejected',[]))"
```

**解决**: 清空 rejected 列表（见一键修复脚本）

### 6.3 Claude Code 无法启动

**检查环境变量是否生效**:

```bash
source ~/.zshrc
env | grep ANTHROPIC
```

---

## 7. 一键修复脚本

### 7.1 完整修复脚本

```bash
#!/bin/bash
# 保存为: ~/fix_claude_proxycast.sh
# 用法: ~/fix_claude_proxycast.sh <YOUR_API_KEY>

API_KEY="${1:-pc_zCRMPk3BhEgIbsH04kDrDL8PZEbbRzsU}"

echo "🔧 开始修复 Claude Code + ProxyCast 配置..."
echo "   使用 API Key: ${API_KEY:0:20}..."

# 1. 修复 ~/.zshrc
echo ""
echo "1️⃣ 更新 ~/.zshrc..."
# 删除旧配置
sed -i '' '/# ProxyCast 代理配置/,/# ======$/d' ~/.zshrc 2>/dev/null
sed -i '' '/ANTHROPIC_AUTH_TOKEN/d' ~/.zshrc 2>/dev/null

# 添加新配置
cat >> ~/.zshrc << EOF

# ======================================================
# ProxyCast 代理配置 - Claude Code 专用
# ======================================================
export ANTHROPIC_AUTH_TOKEN="$API_KEY"
export ANTHROPIC_BASE_URL="http://127.0.0.1:8999"
export ANTHROPIC_API_KEY=""
# ======================================================
EOF
echo "   ✅ ~/.zshrc 已更新"

# 2. 修复 ~/.claude.json
echo ""
echo "2️⃣ 更新 ~/.claude.json..."
python3 << PYEOF
import json
import os

config_path = os.path.expanduser('~/.claude.json')

# 读取现有配置或创建新配置
if os.path.exists(config_path):
    with open(config_path, 'r') as f:
        config = json.load(f)
else:
    config = {}

# 更新 env 配置
config['env'] = {
    "ANTHROPIC_AUTH_TOKEN": "$API_KEY",
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:8999",
    "ANTHROPIC_API_KEY": ""
}

# 清空 rejected 列表
if 'customApiKeyResponses' not in config:
    config['customApiKeyResponses'] = {}
config['customApiKeyResponses']['rejected'] = []
config['customApiKeyResponses']['approved'] = []

# 保存
with open(config_path, 'w') as f:
    json.dump(config, f, indent=2)

print("   ✅ ~/.claude.json 已更新")
PYEOF

# 3. 应用环境变量
echo ""
echo "3️⃣ 应用环境变量..."
export ANTHROPIC_AUTH_TOKEN="$API_KEY"
export ANTHROPIC_BASE_URL="http://127.0.0.1:8999"
export ANTHROPIC_API_KEY=""
echo "   ✅ 当前 session 环境变量已更新"

# 4. 测试连通性
echo ""
echo "4️⃣ 测试 API 连通性..."
RESPONSE=$(curl -s http://127.0.0.1:8999/v1/messages \
  -H "x-api-key: $API_KEY" \
  -H "content-type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{"model": "claude-sonnet-4-5", "max_tokens": 5, "messages": [{"role": "user", "content": "test"}]}' 2>&1)

if echo "$RESPONSE" | grep -q '"content"'; then
    echo "   ✅ API 调用成功!"
else
    echo "   ⚠️ API 调用失败: $RESPONSE"
fi

echo ""
echo "======================================"
echo "🎉 修复完成!"
echo ""
echo "请打开新终端窗口，然后运行: claude"
echo "======================================"
```

### 7.2 使用方法

```bash
# 赋予执行权限
chmod +x ~/fix_claude_proxycast.sh

# 运行修复脚本
~/fix_claude_proxycast.sh pc_zCRMPk3BhEgIbsH04kDrDL8PZEbbRzsU
```

---

## 附录: ProxyCast 支持的模型

| 模型 ID | 说明 |
|---------|------|
| `claude-sonnet-4-5` | Claude Sonnet 4.5 (推荐) |
| `claude-sonnet-4-5-20250929` | Claude Sonnet 4.5 (带日期) |
| `claude-3-7-sonnet-20250219` | Claude 3.7 Sonnet |
| `claude-3-5-sonnet-latest` | Claude 3.5 Sonnet |

> **注意**: Opus 系列模型在 Kiro Free Tier 中不可用，实际会回退到 Sonnet。

---

## 更新日志

- **2025-12-25**: 初始版本，解决 Claude Code v2.0.76 + ProxyCast 认证问题
