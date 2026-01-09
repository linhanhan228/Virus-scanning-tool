#!/bin/bash

# Git Subtree 集成工具脚本
# 用于将外部仓库集成到本地项目中

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_menu() {
    echo -e "${CYAN}[MENU]${NC} $1"
}

# 显示主菜单
show_main_menu() {
    echo ""
    echo "========================================"
    echo "  Git Subtree 集成工具"
    echo "========================================"
    echo ""
    echo "请选择操作："
    echo "  1. 添加外部仓库到项目"
    echo "  2. 更新外部仓库"
    echo "  3. 推送修改到外部仓库"
    echo "  4. 查看已添加的外部仓库"
    echo "  5. 删除外部仓库"
    echo "  6. 退出"
    echo ""
}

# 添加外部仓库
add_external_repo() {
    echo ""
    echo "========================================"
    echo "  添加外部仓库到项目"
    echo "========================================"
    echo ""
    
    read -p "请输入外部仓库URL: " repo_url
    if [ -z "$repo_url" ]; then
        log_error "仓库URL不能为空"
        return 1
    fi
    
    read -p "请输入外部仓库的分支名 (默认: main): " branch_name
    branch_name=${branch_name:-main}
    
    read -p "请输入要放置的目录名: " target_dir
    if [ -z "$target_dir" ]; then
        log_error "目录名不能为空"
        return 1
    fi
    
    read -p "是否压缩提交历史 (y/n, 默认: y): " squash
    squash=${squash:-y}
    
    read -p "请输入远程仓库名称 (默认: external): " remote_name
    remote_name=${remote_name:-external}
    
    echo ""
    log_info "开始添加外部仓库..."
    log_info "仓库URL: $repo_url"
    log_info "分支: $branch_name"
    log_info "目标目录: $target_dir"
    log_info "压缩历史: $squash"
    log_info "远程名称: $remote_name"
    echo ""
    
    read -p "确认添加? (y/n): " confirm
    if [ "$confirm" != "y" ]; then
        log_info "操作已取消"
        return 0
    fi
    
    echo ""
    
    # 检查目标目录是否已存在
    if [ -d "$target_dir" ]; then
        log_error "目标目录 $target_dir 已存在"
        return 1
    fi
    
    # 检查远程仓库是否已存在
    if git remote | grep -q "^${remote_name}$"; then
        log_warn "远程仓库 $remote_name 已存在，将更新URL"
        git remote set-url "$remote_name" "$repo_url"
    else
        git remote add "$remote_name" "$repo_url"
    fi
    
    # 获取外部仓库数据
    log_info "正在获取外部仓库数据..."
    if ! git fetch "$remote_name"; then
        log_error "获取外部仓库数据失败"
        git remote remove "$remote_name" 2>/dev/null || true
        return 1
    fi
    
    # 使用git subtree add
    log_info "正在使用git subtree添加外部仓库..."
    if [ "$squash" = "y" ]; then
        git subtree add --prefix="$target_dir" "$remote_name" "$branch_name" --squash
    else
        git subtree add --prefix="$target_dir" "$remote_name" "$branch_name"
    fi
    
    if [ $? -eq 0 ]; then
        log_info "✓ 外部仓库添加成功！"
        log_info "目录: $target_dir"
        log_info "远程: $remote_name"
        log_info "分支: $branch_name"
        return 0
    else
        log_error "✗ 添加外部仓库失败"
        return 1
    fi
}

# 更新外部仓库
update_external_repo() {
    echo ""
    echo "========================================"
    echo "  更新外部仓库"
    echo "========================================"
    echo ""
    
    # 列出所有远程仓库
    log_info "当前远程仓库："
    git remote -v | grep -v "origin"
    echo ""
    
    read -p "请输入要更新的远程仓库名称: " remote_name
    if [ -z "$remote_name" ]; then
        log_error "远程仓库名称不能为空"
        return 1
    fi
    
    # 查找对应的目标目录
    target_dir=$(git config --local --get "subtree.$remote_name.path" 2>/dev/null || echo "")
    if [ -z "$target_dir" ]; then
        log_error "找不到远程仓库 $remote_name 对应的目录"
        log_warn "请手动输入目标目录名"
        read -p "目标目录名: " target_dir
        if [ -z "$target_dir" ]; then
            log_error "目录名不能为空"
            return 1
        fi
    fi
    
    read -p "请输入要更新的分支名 (默认: main): " branch_name
    branch_name=${branch_name:-main}
    
    read -p "是否压缩提交历史 (y/n, 默认: y): " squash
    squash=${squash:-y}
    
    echo ""
    log_info "开始更新外部仓库..."
    log_info "远程: $remote_name"
    log_info "目录: $target_dir"
    log_info "分支: $branch_name"
    log_info "压缩历史: $squash"
    echo ""
    
    read -p "确认更新? (y/n): " confirm
    if [ "$confirm" != "y" ]; then
        log_info "操作已取消"
        return 0
    fi
    
    echo ""
    
    # 获取最新数据
    log_info "正在获取最新数据..."
    if ! git fetch "$remote_name"; then
        log_error "获取外部仓库数据失败"
        return 1
    fi
    
    # 使用git subtree pull
    log_info "正在使用git subtree pull更新..."
    if [ "$squash" = "y" ]; then
        git subtree pull --prefix="$target_dir" "$remote_name" "$branch_name" --squash
    else
        git subtree pull --prefix="$target_dir" "$remote_name" "$branch_name"
    fi
    
    if [ $? -eq 0 ]; then
        log_info "✓ 外部仓库更新成功！"
        return 0
    else
        log_error "✗ 更新外部仓库失败"
        log_warn "可能需要解决合并冲突"
        return 1
    fi
}

# 推送修改到外部仓库
push_to_external_repo() {
    echo ""
    echo "========================================"
    echo "  推送修改到外部仓库"
    echo "========================================"
    echo ""
    
    # 列出所有远程仓库
    log_info "当前远程仓库："
    git remote -v | grep -v "origin"
    echo ""
    
    read -p "请输入要推送的远程仓库名称: " remote_name
    if [ -z "$remote_name" ]; then
        log_error "远程仓库名称不能为空"
        return 1
    fi
    
    # 查找对应的目标目录
    target_dir=$(git config --local --get "subtree.$remote_name.path" 2>/dev/null || echo "")
    if [ -z "$target_dir" ]; then
        log_error "找不到远程仓库 $remote_name 对应的目录"
        log_warn "请手动输入目标目录名"
        read -p "目标目录名: " target_dir
        if [ -z "$target_dir" ]; then
            log_error "目录名不能为空"
            return 1
        fi
    fi
    
    read -p "请输入要推送的分支名 (默认: main): " branch_name
    branch_name=${branch_name:-main}
    
    echo ""
    log_info "开始推送到外部仓库..."
    log_info "远程: $remote_name"
    log_info "目录: $target_dir"
    log_info "分支: $branch_name"
    echo ""
    
    read -p "确认推送? (y/n): " confirm
    if [ "$confirm" != "y" ]; then
        log_info "操作已取消"
        return 0
    fi
    
    echo ""
    
    # 使用git subtree push
    log_info "正在使用git subtree push推送..."
    git subtree push --prefix="$target_dir" "$remote_name" "$branch_name"
    
    if [ $? -eq 0 ]; then
        log_info "✓ 推送成功！"
        return 0
    else
        log_error "✗ 推送失败"
        return 1
    fi
}

# 查看已添加的外部仓库
list_external_repos() {
    echo ""
    echo "========================================"
    echo "  已添加的外部仓库"
    echo "========================================"
    echo ""
    
    # 列出所有远程仓库（除了origin）
    local repos=$(git remote | grep -v "origin" || true)
    
    if [ -z "$repos" ]; then
        log_info "没有找到已添加的外部仓库"
        return 0
    fi
    
    for remote in $repos; do
        echo "远程名称: $remote"
        git remote get-url "$remote" | sed 's/^/  URL: /'
        
        # 尝试获取对应的目录
        local dir=$(git config --local --get "subtree.$remote.path" 2>/dev/null || echo "未知")
        echo "  目录: $dir"
        echo ""
    done
}

# 删除外部仓库
remove_external_repo() {
    echo ""
    echo "========================================"
    echo "  删除外部仓库"
    echo "========================================"
    echo ""
    
    # 列出所有远程仓库
    log_info "当前远程仓库："
    git remote -v | grep -v "origin"
    echo ""
    
    read -p "请输入要删除的远程仓库名称: " remote_name
    if [ -z "$remote_name" ]; then
        log_error "远程仓库名称不能为空"
        return 1
    fi
    
    # 查找对应的目标目录
    target_dir=$(git config --local --get "subtree.$remote_name.path" 2>/dev/null || echo "")
    
    if [ -z "$target_dir" ]; then
        log_warn "找不到远程仓库 $remote_name 对应的目录"
        read -p "请手动输入要删除的目录名 (留空则只删除远程引用): " target_dir
    fi
    
    echo ""
    log_warn "警告: 此操作将删除远程仓库引用"
    if [ -n "$target_dir" ]; then
        log_warn "并且删除目录: $target_dir"
    fi
    echo ""
    
    read -p "确认删除? (y/n): " confirm
    if [ "$confirm" != "y" ]; then
        log_info "操作已取消"
        return 0
    fi
    
    echo ""
    
    # 删除远程仓库引用
    git remote remove "$remote_name"
    log_info "✓ 已删除远程仓库引用: $remote_name"
    
    # 删除目录
    if [ -n "$target_dir" ] && [ -d "$target_dir" ]; then
        rm -rf "$target_dir"
        log_info "✓ 已删除目录: $target_dir"
        
        # 提交删除
        git add -A
        git commit -m "Remove external repository $remote_name" 2>/dev/null || true
        log_info "✓ 已提交删除操作"
    fi
    
    log_info "✓ 删除操作完成"
    return 0
}

# 主函数
main() {
    # 检查是否在git仓库中
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        log_error "当前目录不是git仓库"
        log_info "请在git仓库目录中运行此脚本"
        exit 1
    fi
    
    while true; do
        show_main_menu
        read -p "请输入选项 (1-6): " choice
        
        case $choice in
            1)
                add_external_repo
                ;;
            2)
                update_external_repo
                ;;
            3)
                push_to_external_repo
                ;;
            4)
                list_external_repos
                ;;
            5)
                remove_external_repo
                ;;
            6)
                log_info "退出程序"
                exit 0
                ;;
            *)
                log_error "无效选项，请重新输入"
                ;;
        esac
        
        echo ""
        read -p "按Enter键继续..." dummy
    done
}

# 运行主函数
main
