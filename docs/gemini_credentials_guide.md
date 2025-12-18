# Gemini 多账号凭证管理指南

## 📋 目录
- [背景知识](#背景知识)
- [凭证结构说明](#凭证结构说明)
- [备份脚本使用](#备份脚本使用)
- [ProxyCast 集成](#proxycast-集成)
- [常见问题](#常见问题)

---

## 背景知识

### Gemini CLI 凭证机制

Gemini CLI **只支持单凭证**，每次 `gemini auth login` 都会覆盖：
```
~/.gemini/oauth_creds.json
```

要使用多账号，需要**手动备份**每个账号的凭证文件。

### 隐藏文件访问

`~/.gemini/` 是隐藏目录，在 Finder 中按 `Cmd + Shift + .` 显示。

---

## 凭证结构说明

```json
{
  "access_token": "ya29...",      // 访问令牌（1小时有效）
  "refresh_token": "1//0g...",    // 刷新令牌（长期有效）
  "token_type": "Bearer",
  "expiry_date": 1766027551684,   // 过期时间戳（毫秒）
  "scope": "...",                 // 权限范围
  "id_token": "eyJ..."            // JWT 身份令牌
}
```

| 字段 | 作用 | 有效期 |
|------|------|--------|
| `access_token` | API 调用认证 | 1 小时 |
| `refresh_token` | 获取新 access_token | 长期有效 |
| `id_token` | 包含用户邮箱等信息 | 1 小时 |

---

## 备份脚本使用

### 脚本位置
```
/Users/ameureka/Desktop/proxycast/scripts/backup_gemini_creds.sh
```

### 命令参考

| 命令 | 说明 |
|------|------|
| `./backup_gemini_creds.sh backup` | 自动识别邮箱并备份 |
| `./backup_gemini_creds.sh backup 名称` | 指定名称备份 |
| `./backup_gemini_creds.sh list` | 列出所有备份 |
| `./backup_gemini_creds.sh restore xxx.json` | 恢复指定备份 |

### 多账号备份流程

```bash
# 步骤 1: 备份当前账号
cd /Users/ameureka/Desktop/proxycast
./scripts/backup_gemini_creds.sh backup

# 步骤 2: 登录另一个账号
gemini auth login
# 在浏览器中完成登录

# 步骤 3: 备份新账号
./scripts/backup_gemini_creds.sh backup

# 步骤 4: 查看备份列表
./scripts/backup_gemini_creds.sh list
```

### 备份文件位置
```
~/Desktop/gemini_backups/
├── wongfeitian_gmail_com_20251218_151609.json
├── another_account_gmail_com_20251218_152000.json
└── ...
```

---

## ProxyCast 集成

### 添加凭证到 ProxyCast

1. 打开 ProxyCast → 「凭证池」→「Gemini (Google)」
2. 点击「添加凭证」
3. 点击「浏览」→ 按 `Cmd + Shift + .` 显示隐藏文件
4. 选择 `~/Desktop/gemini_backups/` 中的备份文件
5. 点击「添加凭证」

### 修复失效凭证

如果凭证显示「读取凭证文件失败」：

1. 点击凭证卡片上的「设置」图标
2. 点击「上传」按钮（Upload 图标）
3. 选择有效的备份文件
4. 点击「保存更改」

---

## 常见问题

### Q: 凭证显示「No such file or directory」？
**原因**: 凭证指向的文件已被删除或覆盖  
**解决**: 删除失效凭证，重新上传备份文件

### Q: Token 刷新失败？
**原因**: `refresh_token` 可能已失效（Google 账号密码变更等）  
**解决**: 重新 `gemini auth login` 并备份

### Q: 多个凭证使用同一账号？
**结果**: 不会冲突，Token 刷新互相独立

### Q: 备份文件可以跨机器使用吗？
**可以**，只要 ProxyCast 配置了正确的 OAuth Client ID/Secret
