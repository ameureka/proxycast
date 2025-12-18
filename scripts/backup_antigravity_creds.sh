#!/bin/bash
# ============================================================
# Antigravity 凭证备份脚本
# 用于在多账号之间切换时备份 OAuth 凭证
# ============================================================

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 路径配置
ANTIGRAVITY_DIR="$HOME/.antigravity"
CREDS_FILE="$ANTIGRAVITY_DIR/oauth_creds.json"
BACKUP_DIR="$HOME/Desktop/antigravity_backups"

# 打印带颜色的消息
print_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
print_success() { echo -e "${GREEN}✅ $1${NC}"; }
print_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
print_error() { echo -e "${RED}❌ $1${NC}"; }

# 显示帮助
show_help() {
    echo ""
    echo "使用方法: $0 [命令]"
    echo ""
    echo "命令:"
    echo "  backup [名称]    备份当前凭证（可选：指定名称）"
    echo "  list             列出所有备份"
    echo "  restore [文件]   恢复指定备份到 ~/.antigravity/"
    echo "  login            启动 Antigravity 登录"
    echo "  help             显示帮助"
    echo ""
    echo "示例:"
    echo "  $0 backup                  # 自动以邮箱命名备份"
    echo "  $0 backup my_work_account  # 指定名称备份"
    echo "  $0 list                    # 列出所有备份"
    echo "  $0 restore account_a.json  # 恢复指定备份"
    echo "  $0 login                   # 登录获取凭证"
    echo ""
}

# 从凭证文件中提取邮箱
get_email_from_creds() {
    if [ -f "$CREDS_FILE" ]; then
        # 尝试解析 id_token 中的邮箱（JWT 格式）
        local id_token=$(cat "$CREDS_FILE" | grep -o '"id_token"[[:space:]]*:[[:space:]]*"[^"]*"' | cut -d'"' -f4)
        if [ -n "$id_token" ]; then
            # JWT 的 payload 是第二部分，base64 解码
            local payload=$(echo "$id_token" | cut -d'.' -f2 | base64 -d 2>/dev/null | tr -d '\0')
            local email=$(echo "$payload" | grep -o '"email"[[:space:]]*:[[:space:]]*"[^"]*"' | cut -d'"' -f4)
            if [ -n "$email" ]; then
                echo "$email"
                return
            fi
        fi
    fi
    echo ""
}

# 登录 Antigravity
do_login() {
    print_info "启动 Antigravity 登录..."
    
    # 检查 Antigravity CLI 是否存在
    if [ ! -f "$HOME/.antigravity/antigravity/bin/antigravity" ]; then
        print_error "Antigravity CLI 未安装"
        print_info "请参考 Antigravity 文档进行安装"
        exit 1
    fi
    
    # 运行登录命令
    "$HOME/.antigravity/antigravity/bin/antigravity" auth login
    
    if [ -f "$CREDS_FILE" ]; then
        print_success "登录成功！凭证已保存"
        print_info "运行 '$0 backup' 备份此凭证"
    else
        print_error "登录后未找到凭证文件"
    fi
}

# 备份凭证
backup_creds() {
    local custom_name="$1"
    
    # 检查凭证文件是否存在
    if [ ! -f "$CREDS_FILE" ]; then
        print_error "凭证文件不存在: $CREDS_FILE"
        print_info "请先运行 '$0 login' 登录"
        exit 1
    fi
    
    # 创建备份目录
    mkdir -p "$BACKUP_DIR"
    
    # 确定备份文件名
    local timestamp=$(date +%Y%m%d_%H%M%S)
    local email=$(get_email_from_creds)
    
    if [ -n "$custom_name" ]; then
        local backup_name="${custom_name}_${timestamp}.json"
    elif [ -n "$email" ]; then
        local safe_email=$(echo "$email" | tr '@.' '_')
        local backup_name="antigravity_${safe_email}_${timestamp}.json"
    else
        local backup_name="antigravity_backup_${timestamp}.json"
    fi
    
    local backup_path="$BACKUP_DIR/$backup_name"
    
    # 复制文件
    cp "$CREDS_FILE" "$backup_path"
    
    print_success "凭证已备份到: $backup_path"
    if [ -n "$email" ]; then
        print_info "账号: $email"
    fi
    
    # 显示下一步提示
    echo ""
    print_info "下一步操作："
    echo "  1. 运行 '$0 login' 登录其他账号"
    echo "  2. 再次运行此脚本备份新账号凭证"
    echo "  3. 在 ProxyCast 中上传这些备份文件"
}

# 列出所有备份
list_backups() {
    if [ ! -d "$BACKUP_DIR" ]; then
        print_warning "备份目录不存在: $BACKUP_DIR"
        exit 0
    fi
    
    echo ""
    echo "═══════════════════════════════════════════════════"
    echo "  📁 Antigravity 凭证备份列表"
    echo "  目录: $BACKUP_DIR"
    echo "═══════════════════════════════════════════════════"
    echo ""
    
    local count=0
    for file in "$BACKUP_DIR"/*.json; do
        if [ -f "$file" ]; then
            count=$((count + 1))
            local filename=$(basename "$file")
            local filesize=$(ls -lh "$file" | awk '{print $5}')
            local filedate=$(stat -f "%Sm" -t "%Y-%m-%d %H:%M" "$file")
            echo "  [$count] $filename"
            echo "      大小: $filesize | 日期: $filedate"
            echo ""
        fi
    done
    
    if [ $count -eq 0 ]; then
        print_warning "没有找到备份文件"
    else
        print_info "共 $count 个备份"
    fi
}

# 恢复备份
restore_backup() {
    local backup_file="$1"
    
    if [ -z "$backup_file" ]; then
        print_error "请指定要恢复的备份文件"
        echo "用法: $0 restore <文件名>"
        exit 1
    fi
    
    # 构建完整路径
    if [ ! -f "$backup_file" ]; then
        backup_file="$BACKUP_DIR/$backup_file"
    fi
    
    if [ ! -f "$backup_file" ]; then
        print_error "备份文件不存在: $backup_file"
        exit 1
    fi
    
    # 确认操作
    print_warning "这将覆盖当前的 Antigravity 凭证！"
    read -p "确认恢复? (y/N): " confirm
    
    if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
        print_info "操作已取消"
        exit 0
    fi
    
    # 确保目录存在
    mkdir -p "$ANTIGRAVITY_DIR"
    
    # 复制文件
    cp "$backup_file" "$CREDS_FILE"
    
    print_success "凭证已恢复！"
    print_info "可以运行 Antigravity 命令验证"
}

# 主逻辑
main() {
    local command="${1:-help}"
    
    case "$command" in
        backup)
            backup_creds "$2"
            ;;
        list)
            list_backups
            ;;
        restore)
            restore_backup "$2"
            ;;
        login)
            do_login
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            print_error "未知命令: $command"
            show_help
            exit 1
            ;;
    esac
}

# 运行
main "$@"
