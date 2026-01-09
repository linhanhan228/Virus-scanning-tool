#!/bin/bash

# ClamAV 项目隔离环境变量
# 此脚本用于设置 ClamAV 相关环境变量，指向本地安装路径
# 使用方法: source clamav_env.sh

CLAMAV_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/local"
DATABASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/database"

export CLAMAV_ROOT
export PATH="${CLAMAV_ROOT}/bin:${CLAMAV_ROOT}/sbin:${PATH}"
export LD_LIBRARY_PATH="${CLAMAV_ROOT}/lib:${LD_LIBRARY_PATH}"
export PKG_CONFIG_PATH="${CLAMAV_ROOT}/lib/pkgconfig:${PKG_CONFIG_PATH}"
export CMAKE_PREFIX_PATH="${CLAMAV_ROOT}:${CMAKE_PREFIX_PATH}"
export ACLOCAL_PATH="${CLAMAV_ROOT}/share/aclocal:${ACLOCAL_PATH}"

export CLAMAV_DATABASE_DIR="${DATABASE_DIR}"
export CLAMAV_LOG_DIR="${CLAMAV_ROOT}/var/log/clamav"
export CLAMAV_CONFIG_DIR="${CLAMAV_ROOT}/etc"

export LD_LIBRARY_PATH="${CLAMAV_ROOT}/lib:$(python3 -c 'import sysconfig; print(sysconfig.get_paths()["purelib"])' 2>/dev/null):${LD_LIBRARY_PATH}"

echo "✓ ClamAV 环境变量已配置"
echo "  CLAMAV_ROOT: ${CLAMAV_ROOT}"
echo "  PATH: ${CLAMAV_ROOT}/bin:${CLAMAV_ROOT}/sbin:..."
echo "  LD_LIBRARY_PATH: ${CLAMAV_ROOT}/lib:..."
echo "  PKG_CONFIG_PATH: ${CLAMAV_ROOT}/lib/pkgconfig:..."
echo ""
echo "病毒库目录: ${CLAMAV_DATABASE_DIR}"
echo "日志目录: ${CLAMAV_LOG_DIR}"
echo "配置目录: ${CLAMAV_CONFIG_DIR}"
echo ""
echo "使用方法:"
echo "  source ${BASH_SOURCE[0]}"
echo ""
