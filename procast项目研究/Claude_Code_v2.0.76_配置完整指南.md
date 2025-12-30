# Claude Code 最新版本 (v2.0.76) 配置完整指南

> **创建日期**: 2025-12-29  
> **Claude Code 版本**: v2.0.76  
> **官方文档**: https://code.claude.com/docs/en/settings

---

## 目录

1. [配置文件层级](#1-配置文件层级)
2. [各配置文件详解](#2-各配置文件详解)
3. [关键环境变量](#3-关键环境变量)
4. [第三方代理配置 (ProxyCast)](#4-第三方代理配置-proxycast)
5. [配置优先级规则](#5-配置优先级规则)
6. [常见问题排查](#6-常见问题排查)
7. [一键诊断脚本](#7-一键诊断脚本)

---

## 1. 配置文件层级

Claude Code 使用**多层配置系统**，优先级从高到低：

| 优先级 | 层级 | 文件路径 | 作用 | 是否共享 |
|:------:|-----|---------|------|:--------:|
| 1 | **企业管理配置** | `/Library/Application Support/ClaudeCode/managed-settings.json` | 由 IT 管理员部署，优先级最高，不可覆盖 | 系统级 |
| 2 | **用户全局设置** | `~/.claude/settings.json` | 用户个人偏好，适用于所有项目 | ❌ |
| 3 | **项目共享设置** | `.claude/settings.json` (项目根目录) | 团队共享配置，提交到 Git | ✅ |
| 4 | **项目本地设置** | `.claude/settings.local.json` (项目根目录) | 个人本地配置，被 Git 忽略 | ❌ |
| 5 | **状态文件** | `~/.claude.json` | 偏好、OAuth、MCP 配置、项目状态、缓存 | ❌ |

### 企业管理配置路径

| 平台 | 路径 |
|-----|------|
| macOS | `/Library/Application Support/ClaudeCode/` |
| Linux / WSL | `/etc/claude-code/` |
| Windows | `C:\Program Files\ClaudeCode\` |

> ⚠️ **注意**: 这些是**系统级路径**（不是用户目录如 `~/Library/...`），需要管理员权限。

---

## 2. 各配置文件详解

### 2.1 `~/.claude/settings.json` (用户全局设置) ⭐

这是**最重要的用户级配置文件**，推荐用于配置代理相关环境变量。

**文件位置**: `~/.claude/settings.json`

**完整示例**:

```json
{
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "pc_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:8999",
    "ANTHROPIC_MODEL": "claude-opus-4-5",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "claude-sonnet-4-5",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "claude-opus-4-5"
  },
  "model": "opus",
  "permissions": {
    "allow": [
      "Bash(npm run *)",
      "Bash(git *)",
      "Read(~/.zshrc)"
    ],
    "deny": [
      "Bash(curl:*)",
      "Read(./.env)",
      "Read(./.env.*)",
      "Read(./secrets/**)"
    ]
  },
  "enabledPlugins": {
    "plugin-name@author": true
  }
}
```

**可用设置项**:

| 字段 | 说明 | 示例 |
|-----|------|------|
| `env` | 环境变量键值对 | `{"FOO": "bar"}` |
| `model` | 默认模型 | `"claude-sonnet-4-5-20250929"` |
| `permissions` | 权限规则 | `{"allow": [...], "deny": [...]}` |
| `hooks` | Hook 配置 | `{"PreToolUse": {"Bash": "echo 'Running...'"}}` |
| `outputStyle` | 输出风格 | `"Explanatory"` |
| `cleanupPeriodDays` | 清理周期 | `20` (0 表示禁用) |
| `forceLoginMethod` | 强制登录方式 | `"claudeai"` 或 `"console"` |

---

### 2.2 `~/.claude.json` (状态文件)

这个文件由 Claude Code **自动管理**，通常不需要手动编辑。

**文件位置**: `~/.claude.json`

**存储内容**:
- OAuth 会话信息
- 用户偏好（主题、通知设置、编辑器模式）
- MCP 服务器配置（user 和 local 作用域）
- 项目级状态（允许的工具、trust 设置）
- 各种缓存

**关键字段示例**:

```json
{
  "theme": "light-daltonized",
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "pc_xxx...",
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:8999"
  },
  "customApiKeyResponses": {
    "approved": [],
    "rejected": []
  },
  "projects": {
    "/path/to/project": {
      "allowedTools": [],
      "hasTrustDialogAccepted": true,
      "mcpServers": {}
    }
  }
}
```

> ⚠️ **重要**: `customApiKeyResponses.rejected` 数组中的 Key 会被拒绝使用。如果你的 Key 被误拒，需要清空此数组。

---

### 2.3 项目级配置

#### `.claude/settings.json` (团队共享)

放在项目根目录，会被提交到 Git，适合团队共享的规则：

```json
{
  "permissions": {
    "allow": ["Bash(npm run lint)", "Bash(npm run test:*)"],
    "deny": ["Read(./.env)"]
  }
}
```

#### `.claude/settings.local.json` (个人本地)

同样放在项目根目录，但 Claude Code 会自动配置 Git 忽略此文件：

```json
{
  "env": {
    "MY_PERSONAL_TOKEN": "xxx"
  }
}
```

---

### 2.4 MCP 服务器配置

| 作用域 | 配置位置 |
|-------|---------|
| user | `~/.claude.json` 中的 `mcpServers` |
| local | `~/.claude.json` 中的项目级 `mcpServers` |
| project | `.mcp.json` (项目根目录，提交到 Git) |

---

## 3. 关键环境变量

### 3.1 认证相关

| 变量名 | 作用 | HTTP 头 | 使用场景 |
|-------|------|--------|---------|
| `ANTHROPIC_API_KEY` | 官方 API Key | `X-Api-Key: <value>` | 直连 Anthropic 官方 API |
| `ANTHROPIC_AUTH_TOKEN` | 第三方代理 Token | `Authorization: Bearer <value>` | **ProxyCast 等代理服务** |
| `ANTHROPIC_BASE_URL` | API 基础 URL | - | 指向代理服务器地址 |

> 💡 **关键区别**: 使用第三方代理时，必须使用 `ANTHROPIC_AUTH_TOKEN` 而不是 `ANTHROPIC_API_KEY`！

### 3.2 模型相关

| 变量名 | 说明 |
|-------|------|
| `ANTHROPIC_MODEL` | 覆盖默认模型 |
| `ANTHROPIC_DEFAULT_OPUS_MODEL` | Opus 模型别名 |
| `ANTHROPIC_DEFAULT_SONNET_MODEL` | Sonnet 模型别名 |
| `ANTHROPIC_DEFAULT_HAIKU_MODEL` | Haiku 模型别名 |
| `ANTHROPIC_SMALL_FAST_MODEL` | 后台任务使用的 Haiku 类模型 |

### 3.3 代理和网络

| 变量名 | 说明 |
|-------|------|
| `HTTP_PROXY` | HTTP 代理地址 |
| `HTTPS_PROXY` | HTTPS 代理地址 |
| `NO_PROXY` | 不走代理的主机列表 |

### 3.4 其他重要变量

| 变量名 | 说明 |
|-------|------|
| `CLAUDE_CONFIG_DIR` | 自定义配置目录 |
| `DISABLE_AUTOUPDATER` | 设为 `1` 禁用自动更新 |
| `DISABLE_TELEMETRY` | 设为 `1` 禁用遥测 |
| `BASH_DEFAULT_TIMEOUT_MS` | Bash 命令默认超时 |
| `MCP_TIMEOUT` | MCP 服务器启动超时 |

---

## 4. 第三方代理配置 (ProxyCast)

### 4.1 配置要点

使用 ProxyCast 等第三方代理时，需要注意：

1. **使用 `ANTHROPIC_AUTH_TOKEN`** 而不是 `ANTHROPIC_API_KEY`
2. **设置 `ANTHROPIC_BASE_URL`** 指向本地代理
3. **确保 API Key 一致**: 客户端配置的 Key 必须与 ProxyCast 服务端 `config.yaml` 中的 `api_key` 完全一致

### 4.2 推荐配置方法

#### 方法一：通过 `~/.claude/settings.json` (推荐)

```json
{
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "pc_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:8999",
    "ANTHROPIC_MODEL": "claude-opus-4-5"
  }
}
```

#### 方法二：通过 `~/.zshrc`

```bash
# ======================================================
# ProxyCast 代理配置 - Claude Code 专用
# ======================================================
# 关键：使用 ANTHROPIC_AUTH_TOKEN 而不是 ANTHROPIC_API_KEY
export ANTHROPIC_AUTH_TOKEN="pc_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
export ANTHROPIC_BASE_URL="http://127.0.0.1:8999"
# 必须设为空，防止 Claude Code 尝试官方认证
export ANTHROPIC_API_KEY=""
# ======================================================
```

### 4.3 ProxyCast 服务端配置

**文件**: `~/Library/Application Support/proxycast/config.yaml`

```yaml
server:
  host: 127.0.0.1
  port: 8999
  api_key: pc_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx  # 客户端必须匹配此 Key
```

---

## 5. 配置优先级规则

### 5.1 环境变量来源优先级

Claude Code 读取环境变量的优先级如下：

1. `~/.claude.json` 中的 `env` 配置 (**最高优先级**)
2. `~/.claude/settings.json` 中的 `env` 配置
3. `.claude/settings.local.json` 中的 `env` 配置
4. `.claude/settings.json` 中的 `env` 配置
5. Shell 环境变量 (如 `~/.zshrc`)
6. macOS Keychain (官方 OAuth)

### 5.2 配置合并规则

- **settings.json 配置会合并**，而不是完全覆盖
- **数组会合并**（如 permissions.allow）
- **对象会递归合并**（如 env）
- **更高优先级的值会覆盖**

---

## 6. 常见问题排查

### 6.1 401 Invalid API key

**原因**: 客户端发送的 API Key 与 ProxyCast 服务端配置不匹配

**排查步骤**:

```bash
# 1. 查看 ProxyCast 服务端的 API Key
cat ~/Library/Application\ Support/proxycast/config.yaml | grep api_key

# 2. 查看 ~/.claude/settings.json 中的 Key
cat ~/.claude/settings.json | python3 -c "import json,sys; print(json.load(sys.stdin).get('env',{}).get('ANTHROPIC_AUTH_TOKEN','未设置'))"

# 3. 查看 ~/.claude.json 中的 Key
cat ~/.claude.json | python3 -c "import json,sys; print(json.load(sys.stdin).get('env',{}).get('ANTHROPIC_AUTH_TOKEN','未设置'))"

# 4. 查看环境变量
echo $ANTHROPIC_AUTH_TOKEN
```

**解决**: 确保所有位置的 API Key 完全一致

### 6.2 API Key 被拒绝

**原因**: Key 被添加到了 `~/.claude.json` 的 rejected 列表

**排查**:

```bash
cat ~/.claude.json | python3 -c "import json,sys; print(json.load(sys.stdin).get('customApiKeyResponses',{}).get('rejected',[]))"
```

**解决**: 清空 rejected 列表

```bash
# 使用 Python 清除 rejected 列表
python3 << 'EOF'
import json
with open('$HOME/.claude.json', 'r') as f:
    config = json.load(f)
if 'customApiKeyResponses' in config:
    config['customApiKeyResponses']['rejected'] = []
with open('$HOME/.claude.json', 'w') as f:
    json.dump(config, f, indent=2)
print("✅ rejected 列表已清空")
EOF
```

### 6.3 配置不生效

**检查步骤**:

1. 配置文件 JSON 语法是否正确
2. 是否有更高优先级的配置覆盖
3. 是否重新启动了 Claude Code

---

## 7. 一键诊断脚本

保存为 `~/check_claude_config.sh`：

```bash
#!/bin/bash

echo "======================================"
echo "Claude Code 配置诊断 (v2.0.76)"
echo "======================================"

# 1. 检查配置文件存在性
echo ""
echo "1️⃣ 配置文件检查:"
[ -f ~/.claude/settings.json ] && echo "   ✅ ~/.claude/settings.json" || echo "   ❌ ~/.claude/settings.json 不存在"
[ -f ~/.claude.json ] && echo "   ✅ ~/.claude.json" || echo "   ❌ ~/.claude.json 不存在"

# 2. 检查 ~/.claude/settings.json
echo ""
echo "2️⃣ ~/.claude/settings.json 环境变量:"
if [ -f ~/.claude/settings.json ]; then
    python3 -c "
import json
with open('$HOME/.claude/settings.json') as f:
    d = json.load(f)
env = d.get('env', {})
print('   AUTH_TOKEN:', env.get('ANTHROPIC_AUTH_TOKEN', '[未设置]')[:30] + '...' if env.get('ANTHROPIC_AUTH_TOKEN') else '[未设置]')
print('   BASE_URL:', env.get('ANTHROPIC_BASE_URL', '[未设置]'))
print('   API_KEY:', repr(env.get('ANTHROPIC_API_KEY', '[未设置]')))
"
fi

# 3. 检查 ~/.claude.json
echo ""
echo "3️⃣ ~/.claude.json 环境变量:"
if [ -f ~/.claude.json ]; then
    python3 -c "
import json
with open('$HOME/.claude.json') as f:
    d = json.load(f)
env = d.get('env', {})
print('   AUTH_TOKEN:', env.get('ANTHROPIC_AUTH_TOKEN', '[未设置]')[:30] + '...' if env.get('ANTHROPIC_AUTH_TOKEN') else '[未设置]')
print('   BASE_URL:', env.get('ANTHROPIC_BASE_URL', '[未设置]'))
"

    # 检查 rejected 列表
    echo ""
    echo "4️⃣ 被拒绝的 API Key:"
    REJECTED=$(python3 -c "import json; print(json.load(open('$HOME/.claude.json')).get('customApiKeyResponses',{}).get('rejected',[]))")
    if [ "$REJECTED" = "[]" ]; then
        echo "   ✅ 无被拒绝的 Key"
    else
        echo "   ⚠️ 存在被拒绝的 Key: $REJECTED"
    fi
fi

# 4. 检查 Shell 环境变量
echo ""
echo "5️⃣ Shell 环境变量:"
echo "   ANTHROPIC_AUTH_TOKEN=${ANTHROPIC_AUTH_TOKEN:+${ANTHROPIC_AUTH_TOKEN:0:20}...}"
echo "   ANTHROPIC_BASE_URL=${ANTHROPIC_BASE_URL:-[未设置]}"
echo "   ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY:-[未设置]}"

# 5. 检查 ProxyCast
echo ""
echo "6️⃣ ProxyCast 配置:"
if [ -f ~/Library/Application\ Support/proxycast/config.yaml ]; then
    PROXYCAST_KEY=$(grep "api_key:" ~/Library/Application\ Support/proxycast/config.yaml | awk '{print $2}')
    echo "   服务端 API Key: ${PROXYCAST_KEY:0:20}..."
else
    echo "   ❌ ProxyCast config.yaml 不存在"
fi

# 6. 测试 ProxyCast 连通性
echo ""
echo "7️⃣ ProxyCast 服务状态:"
if curl -s http://127.0.0.1:8999/health | grep -q "healthy" 2>/dev/null; then
    echo "   ✅ ProxyCast 运行正常"
else
    echo "   ❌ ProxyCast 未运行或不可达"
fi

echo ""
echo "======================================"
echo "诊断完成"
echo "======================================"
```

使用方法：

```bash
chmod +x ~/check_claude_config.sh
~/check_claude_config.sh
```

---

## 附录：配置文件路径速查表

| 配置类型 | macOS 路径 | Linux/WSL 路径 | Windows 路径 |
|---------|-----------|---------------|-------------|
| 用户设置 | `~/.claude/settings.json` | `~/.claude/settings.json` | `%USERPROFILE%\.claude\settings.json` |
| 状态文件 | `~/.claude.json` | `~/.claude.json` | `%USERPROFILE%\.claude.json` |
| 项目设置 | `.claude/settings.json` | `.claude/settings.json` | `.claude\settings.json` |
| 项目本地设置 | `.claude/settings.local.json` | `.claude/settings.local.json` | `.claude\settings.local.json` |
| MCP 配置 | `.mcp.json` | `.mcp.json` | `.mcp.json` |
| ProxyCast | `~/Library/Application Support/proxycast/config.yaml` | `~/.config/proxycast/config.yaml` | `%APPDATA%\proxycast\config.yaml` |

---

## 参考资料

- [Claude Code 官方设置文档](https://code.claude.com/docs/en/settings)
- [Claude Code 模型配置](https://code.claude.com/docs/en/model-config)
- [Claude Code 环境变量](https://code.claude.com/docs/en/settings#environment-variables)
- [ProxyCast 官方文档](https://aiclientproxy.github.io/proxycast/)

---

## 更新日志

- **2025-12-29**: 基于 Claude Code v2.0.76 创建初始版本
