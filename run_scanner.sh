#!/bin/bash

# ClamAV 扫描脚本
# 使用本地安装的 ClamAV 扫描文件或目录

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOCAL_DIR="${SCRIPT_DIR}/local"
ENV_SCRIPT="${SCRIPT_DIR}/clamav_env.sh"
DATABASE_DIR="${SCRIPT_DIR}/database"

# 加载环境变量
if [ -f "${ENV_SCRIPT}" ]; then
    source "${ENV_SCRIPT}"
else
    export CLAMAV_ROOT="${LOCAL_DIR}"
    export PATH="${LOCAL_DIR}/bin:${LOCAL_DIR}/sbin:${PATH}"
    export LD_LIBRARY_PATH="${LOCAL_DIR}/lib:${LD_LIBRARY_PATH}"
    export CLAMAV_DATABASE_DIR="${DATABASE_DIR}"
fi

CLAMSCAN="${LOCAL_DIR}/bin/clamscan"

# 检查病毒库是否存在
if [ ! -d "${CLAMAV_DATABASE_DIR}" ] || [ -z "$(ls -A "${CLAMAV_DATABASE_DIR}" 2>/dev/null)" ]; then
    echo "✗ 病毒库为空或不存在"
    echo "请先运行: ${SCRIPT_DIR}/update_clamav_db.sh"
    exit 1
fi

# 显示帮助信息
show_help() {
    echo "用法: $0 [选项] <路径>"
    echo ""
    echo "选项:"
    echo "  -h, --help           显示帮助信息"
    echo "  -r, --recursive      递归扫描目录"
    echo "  -v, --verbose        显示详细信息"
    echo "  --move=<目录>        移动感染文件到指定目录"
    echo "  --copy=<目录>        复制感染文件到指定目录"
    echo "  --exclude=<模式>     排除匹配的文件"
    echo "  --exclude-dir=<模式> 排除匹配的目录"
    echo ""
    echo "示例:"
    echo "  $0 /path/to/scan           扫描单个文件"
    echo "  $0 -r /path/to/scan        递归扫描目录"
    echo "  $0 -r --move=./infected /path/to/scan"
}

# 解析参数
RECURSIVE=""
VERBOSE=""
MOVE_DIR=""
COPY_DIR=""
EXCLUDE=""
EXCLUDE_DIR=""

while [ $# -gt 0 ]; do
    case "$1" in
        -h|--help)
            show_help
            exit 0
            ;;
        -r|--recursive)
            RECURSIVE="--recursive"
            shift
            ;;
        -v|--verbose)
            VERBOSE="--verbose"
            shift
            ;;
        --move=*)
            MOVE_DIR="${1#*=}"
            shift
            ;;
        --copy=*)
            COPY_DIR="${1#*=}"
            shift
            ;;
        --exclude=*)
            EXCLUDE="--exclude=${1#*=}"
            shift
            ;;
        --exclude-dir=*)
            EXCLUDE_DIR="--exclude-dir=${1#*=}"
            shift
            ;;
        -*)
            echo "未知选项: $1"
            show_help
            exit 1
            ;;
        *)
            SCAN_PATH="$1"
            shift
            ;;
    esac
done

# 检查扫描路径
if [ -z "${SCAN_PATH}" ]; then
    echo "✗ 未指定扫描路径"
    show_help
    exit 1
fi

if [ ! -e "${SCAN_PATH}" ]; then
    echo "✗ 路径不存在: ${SCAN_PATH}"
    exit 1
fi

# 构建扫描参数
SCAN_ARGS=""
[ -n "${RECURSIVE}" ] && SCAN_ARGS="${SCAN_ARGS} ${RECURSIVE}"
[ -n "${VERBOSE}" ] && SCAN_ARGS="${SCAN_ARGS} ${VERBOSE}"
[ -n "${MOVE_DIR}" ] && SCAN_ARGS="${SCAN_ARGS} --move=${MOVE_DIR}"
[ -n "${COPY_DIR}" ] && SCAN_ARGS="${SCAN_ARGS} --copy=${COPY_DIR}"
[ -n "${EXCLUDE}" ] && SCAN_ARGS="${SCAN_ARGS} ${EXCLUDE}"
[ -n "${EXCLUDE_DIR}" ] && SCAN_ARGS="${SCAN_ARGS} ${EXCLUDE_DIR}"

echo "========================================"
echo "  ClamAV 病毒扫描"
echo "========================================"
echo ""
echo "扫描路径: ${SCAN_PATH}"
echo "病毒库: ${CLAMAV_DATABASE_DIR}"
echo ""

# 执行扫描
"${CLAMSCAN}" ${SCAN_ARGS} "${SCAN_PATH}"

echo ""
echo "✓ 扫描完成"
