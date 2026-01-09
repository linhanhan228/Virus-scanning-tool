#!/bin/bash

# ClamAV 病毒库更新脚本
# 使用本地安装的 ClamAV 工具更新病毒库

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
    export CLAMAV_LOG_DIR="${LOCAL_DIR}/var/log/clamav"
fi

# 配置路径
FRESHCLAM_CONF="${LOCAL_DIR}/etc/freshclam.conf.sample"
LOG_DIR="${LOCAL_DIR}/var/log/clamav"

# 创建必要目录
mkdir -p "${DATABASE_DIR}"
mkdir -p "${LOG_DIR}"

echo "========================================"
echo "  ClamAV 病毒库更新"
echo "========================================"
echo ""
echo "数据库目录: ${DATABASE_DIR}"
echo ""

# 使用 freshclam 更新病毒库
if [ -f "${FRESHCLAM_CONF}" ]; then
    # 创建临时配置
    local temp_conf=$(mktemp)
    cp "${FRESHCLAM_CONF}" "${temp_conf}"

    # 修改配置中的路径
    sed -i '' "s|^DatabaseDirectory.*|DatabaseDirectory ${DATABASE_DIR}|g" "${temp_conf}"
    sed -i '' "s|^LogFile.*|LogFile ${LOG_DIR}/freshclam.log|g" "${temp_conf}"
    sed -i '' "s|^DNSDatabaseInfo.*|DNSDatabaseInfo current.cvd.clamav.net|g" "${temp_conf}"
    sed -i '' "s|^PrivateMirror.*|PrivateMirror database.clamav.net|g" "${temp_conf}"

    echo "开始下载病毒库..."
    "${LOCAL_DIR}/bin/freshclam" --config-file="${temp_conf}"

    rm -f "${temp_conf}"

    echo ""
    echo "✓ 病毒库更新完成"
    echo "病毒库位置: ${DATABASE_DIR}"
    ls -lh "${DATABASE_DIR}"/*.cvd 2>/dev/null || ls -lh "${DATABASE_DIR}"/*.cdiff 2>/dev/null || echo "等待下载完成..."
else
    echo "✗ 找不到 freshclam 配置文件"
    exit 1
fi
