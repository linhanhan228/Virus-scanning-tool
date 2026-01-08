#!/bin/bash

# 病毒查杀工具功能测试脚本
# 用于测试所有命令行功能是否完整且不报错

set -e  # 遇到错误立即退出

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 测试计数器
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

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

# 测试函数
run_test() {
    local test_name="$1"
    local command="$2"
    local should_fail="$3"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    echo ""
    echo "========================================"
    echo "测试 $TOTAL_TESTS: $test_name"
    echo "========================================"
    echo "执行命令: $command"
    echo ""
    
    if [ "$should_fail" = "true" ]; then
        if eval "$command" > /dev/null 2>&1; then
            log_error "测试失败: 命令应该失败但成功了"
            FAILED_TESTS=$((FAILED_TESTS + 1))
            return 1
        else
            log_info "测试通过: 命令按预期失败"
            PASSED_TESTS=$((PASSED_TESTS + 1))
            return 0
        fi
    else
        if eval "$command"; then
            log_info "测试通过: 命令执行成功"
            PASSED_TESTS=$((PASSED_TESTS + 1))
            return 0
        else
            log_error "测试失败: 命令执行失败"
            FAILED_TESTS=$((FAILED_TESTS + 1))
            return 1
        fi
    fi
}

# 获取可执行文件路径
get_binary_path() {
    if [ -f "./target/release/virus-scanner" ]; then
        echo "./target/release/virus-scanner"
    elif [ -f "./target/debug/virus-scanner" ]; then
        echo "./target/debug/virus-scanner"
    else
        log_error "找不到可执行文件，请先编译项目"
        exit 1
    fi
}

# 获取配置文件路径
get_config_path() {
    if [ -f "./etc/config.yaml" ]; then
        echo "./etc/config.yaml"
    else
        log_error "找不到配置文件: ./etc/config.yaml"
        exit 1
    fi
}

# 创建测试目录
setup_test_environment() {
    log_info "设置测试环境..."
    
    # 创建必要的目录
    mkdir -p /tmp/virus-scanner-test
    mkdir -p /tmp/virus-scanner-test/database
    mkdir -p /tmp/virus-scanner-test/backups
    mkdir -p /tmp/virus-scanner-test/quarantine
    mkdir -p /tmp/virus-scanner-test/reports
    mkdir -p /tmp/virus-scanner-test/logs
    
    # 创建系统级目录（用于报告输出）
    sudo mkdir -p /var/lib/virus-scanner/reports 2>/dev/null || mkdir -p /var/lib/virus-scanner/reports 2>/dev/null || true
    
    # 创建测试文件
    echo "test content" > /tmp/virus-scanner-test/test1.txt
    echo "another test" > /tmp/virus-scanner-test/test2.txt
    
    log_info "测试环境设置完成"
}

# 清理测试环境
cleanup_test_environment() {
    log_info "清理测试环境..."
    rm -rf /tmp/virus-scanner-test
    log_info "测试环境清理完成"
}

# 主测试流程
main() {
    echo ""
    echo "========================================"
    echo "  病毒查杀工具功能测试脚本"
    echo "========================================"
    echo ""
    
    # 获取可执行文件路径
    BINARY=$(get_binary_path)
    log_info "使用可执行文件: $BINARY"
    
    # 获取配置文件路径
    CONFIG=$(get_config_path)
    log_info "使用配置文件: $CONFIG"
    
    # 检查可执行文件是否存在
    if [ ! -f "$BINARY" ]; then
        log_error "可执行文件不存在: $BINARY"
        exit 1
    fi
    
    # 检查配置文件是否存在
    if [ ! -f "$CONFIG" ]; then
        log_error "配置文件不存在: $CONFIG"
        exit 1
    fi
    
    # 设置测试环境
    setup_test_environment
    
    # 测试 1: 显示版本信息
    run_test "显示版本信息" "$BINARY --config $CONFIG --version"
    
    # 测试 2: 显示帮助信息
    run_test "显示帮助信息" "$BINARY --config $CONFIG --help"
    
    # 测试 3: 显示扫描帮助
    run_test "显示扫描帮助" "$BINARY --config $CONFIG scan --help"
    
    # 测试 4: 显示更新帮助
    run_test "显示更新帮助" "$BINARY --config $CONFIG update --help"
    
    # 测试 5: 显示监控帮助
    run_test "显示监控帮助" "$BINARY --config $CONFIG monitor --help"
    
    # 测试 6: 显示报告帮助
    run_test "显示报告帮助" "$BINARY --config $CONFIG report --help"
    
    # 测试 7: 显示状态帮助
    run_test "显示状态帮助" "$BINARY --config $CONFIG status --help"
    
    # 测试 8: 检查病毒库状态
    run_test "检查病毒库状态" "$BINARY --config $CONFIG status --database"
    
    # 测试 9: 检查系统状态
    run_test "检查系统状态" "$BINARY --config $CONFIG status --system"
    
    # 测试 10: 快速扫描（测试模式）
    run_test "快速扫描测试" "$BINARY --config $CONFIG scan --scan-type quick --report --format text"
    
    # 测试 11: 自定义路径扫描
    run_test "自定义路径扫描" "$BINARY --config $CONFIG scan --scan-type custom --paths /tmp/virus-scanner-test --report --format json"
    
    # 测试 12: 检查病毒库更新
    run_test "检查病毒库更新" "$BINARY --config $CONFIG update --check-only"
    
    # 测试 13: 测试无效命令（应该失败）
    run_test "测试无效命令" "$BINARY --config $CONFIG invalid-command" "true"
    
    # 测试 14: 测试无效参数（应该失败）
    run_test "测试无效参数" "$BINARY --config $CONFIG scan --invalid-param" "true"
    
    # 测试 15: 测试详细输出模式
    run_test "测试详细输出模式" "$BINARY --config $CONFIG status --database --verbose"
    
    # 测试 16: 测试扫描排除路径
    run_test "测试扫描排除路径" "$BINARY --config $CONFIG scan --scan-type custom --paths /tmp/virus-scanner-test --exclude /tmp/virus-scanner-test/test1.txt"
    
    # 测试 17: 测试不同报告格式
    run_test "测试JSON报告格式" "$BINARY --config $CONFIG scan --scan-type quick --report --format json"
    
    # 测试 18: 测试YAML报告格式
    run_test "测试YAML报告格式" "$BINARY --config $CONFIG scan --scan-type quick --report --format yaml"
    
    # 测试 19: 测试HTML报告格式
    run_test "测试HTML报告格式" "$BINARY --config $CONFIG scan --scan-type quick --report --format html"
    
    # 测试 20: 测试TEXT报告格式
    run_test "测试TEXT报告格式" "$BINARY --config $CONFIG scan --scan-type quick --report --format text"
    
    # 清理测试环境
    cleanup_test_environment
    
    # 打印测试结果
    echo ""
    echo "========================================"
    echo "  测试结果汇总"
    echo "========================================"
    echo "总测试数: $TOTAL_TESTS"
    echo -e "${GREEN}通过: $PASSED_TESTS${NC}"
    echo -e "${RED}失败: $FAILED_TESTS${NC}"
    echo ""
    
    if [ $FAILED_TESTS -eq 0 ]; then
        echo -e "${GREEN}✓ 所有测试通过！${NC}"
        exit 0
    else
        echo -e "${RED}✗ 有 $FAILED_TESTS 个测试失败${NC}"
        exit 1
    fi
}

# 运行主函数
main
