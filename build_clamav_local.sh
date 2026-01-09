#!/bin/bash

# ClamAV 病毒库本地化编译安装脚本
# 编译并安装到项目专属路径，不进行系统级全局安装

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 项目路径配置
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLAMAV_SOURCE_DIR="${PROJECT_ROOT}/clamav"
LOCAL_INSTALL_DIR="${PROJECT_ROOT}/local"
BUILD_DIR="${PROJECT_ROOT}/build"
DATABASE_DIR="${PROJECT_ROOT}/database"

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

log_step() {
    echo -e "${CYAN}[STEP]${NC} $1"
}

# 显示帮助信息
show_help() {
    echo ""
    echo "========================================"
    echo "  ClamAV 本地化编译安装脚本"
    echo "========================================"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  --help          显示帮助信息"
    echo "  --clean         清理构建目录和安装文件"
    echo "  --build-only    仅编译，不安装"
    echo "  --install-only  仅安装（假设已编译）"
    echo "  --verify        验证安装结果"
    echo "  --full          完整流程（清理、编译、安装、验证）"
    echo ""
    echo "安装路径: ${LOCAL_INSTALL_DIR}"
    echo ""
}

# 检查系统依赖
check_dependencies() {
    log_step "检查系统依赖..."

    local missing_deps=()

    # 检查必需的命令
    for cmd in cmake make gcc g++; do
        if ! command -v $cmd &> /dev/null; then
            missing_deps+=("$cmd")
        fi
    done

    # 检查 Rust (某些版本需要)
    if ! command -v cargo &> /dev/null && ! command -v rustc &> /dev/null; then
        log_warn "Rust 工具链未找到，将使用 CMake 原生编译"
    fi

    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "缺少必要的依赖: ${missing_deps[*]}"
        log_info "请安装后再运行此脚本"
        exit 1
    fi

    log_info "✓ 系统依赖检查通过"
}

# 检查源码目录
check_source() {
    log_step "检查 ClamAV 源码..."

    if [ ! -d "${CLAMAV_SOURCE_DIR}" ]; then
        log_error "找不到 ClamAV 源码目录: ${CLAMAV_SOURCE_DIR}"
        exit 1
    fi

    if [ ! -f "${CLAMAV_SOURCE_DIR}/CMakeLists.txt" ]; then
        log_error "CMakeLists.txt 未找到，请确保这是有效的 ClamAV 源码"
        exit 1
    fi

    log_info "✓ ClamAV 源码检查通过"
}

# 创建本地安装目录结构
create_local_dirs() {
    log_step "创建本地安装目录结构..."

    mkdir -p "${LOCAL_INSTALL_DIR}"
    mkdir -p "${LOCAL_INSTALL_DIR}/bin"
    mkdir -p "${LOCAL_INSTALL_DIR}/sbin"
    mkdir -p "${LOCAL_INSTALL_DIR}/lib"
    mkdir -p "${LOCAL_INSTALL_DIR}/lib/pkgconfig"
    mkdir -p "${LOCAL_INSTALL_DIR}/include"
    mkdir -p "${LOCAL_INSTALL_DIR}/share"
    mkdir -p "${LOCAL_INSTALL_DIR}/etc"
    mkdir -p "${DATABASE_DIR}"
    mkdir -p "${LOCAL_INSTALL_DIR}/var/log/clamav"

    log_info "✓ 本地安装目录已创建"
    log_info "  安装路径: ${LOCAL_INSTALL_DIR}"
    log_info "  病毒库路径: ${DATABASE_DIR}"
}

# 清理构建目录
clean_build() {
    log_step "清理构建目录..."

    if [ -d "${BUILD_DIR}" ]; then
        rm -rf "${BUILD_DIR}"
        log_info "✓ 已清理构建目录: ${BUILD_DIR}"
    else
        log_info "构建目录不存在，无需清理"
    fi
}

# 配置 CMake
configure_cmake() {
    log_step "配置 CMake..."

    mkdir -p "${BUILD_DIR}"
    cd "${BUILD_DIR}"

    # 配置 CMake，使用本地安装路径
    cmake \
        -DCMAKE_INSTALL_PREFIX="${LOCAL_INSTALL_DIR}" \
        -DCMAKE_BUILD_TYPE=Release \
        -DENABLE_APP=ON \
        -DENABLE_CLAMONACC=OFF \
        -DENABLE_MILTER=OFF \
        -DENABLE_TESTS=OFF \
        -DENABLE_DOCS=OFF \
        -DBUILD_SHARED_LIBS=OFF \
        -DCLAMAV_USER=root \
        -DCLAMAV_GROUP=wheel \
        -D病原体_LIBRARY_DIR="${LOCAL_INSTALL_DIR}/lib" \
        -D病原体_INCLUDE_DIR="${LOCAL_INSTALL_DIR}/include" \
        "${CLAMAV_SOURCE_DIR}"

    if [ $? -eq 0 ]; then
        log_info "✓ CMake 配置成功"
    else
        log_error "✗ CMake 配置失败"
        exit 1
    fi
}

# 编译 ClamAV
build_clamav() {
    log_step "编译 ClamAV..."

    cd "${BUILD_DIR}"

    # 获取 CPU 核心数
    local cpu_cores=$(sysctl -n hw.ncpu 2>/dev/null || echo 4)

    make -j${cpu_cores}

    if [ $? -eq 0 ]; then
        log_info "✓ ClamAV 编译成功"
    else
        log_error "✗ ClamAV 编译失败"
        exit 1
    fi
}

# 安装到本地目录
install_clamav() {
    log_step "安装 ClamAV 到本地目录..."

    cd "${BUILD_DIR}"

    # 仅安装，不使用 sudo
    make install

    if [ $? -eq 0 ]; then
        log_info "✓ ClamAV 安装成功"
    else
        log_error "✗ ClamAV 安装失败"
        exit 1
    fi
}

# 创建项目隔离的环境变量脚本
create_env_script() {
    log_step "创建项目隔离的环境变量脚本..."

    local env_script="${PROJECT_ROOT}/clamav_env.sh"

    cat > "${env_script}" << 'ENVEOF'
#!/bin/bash

# ClamAV 项目隔离环境变量
# 此脚本用于设置 ClamAV 相关环境变量，指向本地安装路径
# 使用方法: source clamav_env.sh

export CLAMAV_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/local"
export PATH="${CLAMAV_ROOT}/bin:${CLAMAV_ROOT}/sbin:${PATH}"
export LD_LIBRARY_PATH="${CLAMAV_ROOT}/lib:${LD_LIBRARY_PATH}"
export PKG_CONFIG_PATH="${CLAMAV_ROOT}/lib/pkgconfig:${PKG_CONFIG_PATH}"
export CMAKE_PREFIX_PATH="${CLAMAV_ROOT}:${CMAKE_PREFIX_PATH}"
export ACLOCAL_PATH="${CLAMAV_ROOT}/share/aclocal:${ACLOCAL_PATH}"

export CLAMAV_DATABASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/database"
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
ENVEOF

    chmod +x "${env_script}"
    log_info "✓ 环境变量脚本已创建: ${env_script}"
}

# 创建病毒库下载脚本
create_db_update_script() {
    log_step "创建病毒库更新脚本..."

    local db_script="${PROJECT_ROOT}/update_clamav_db.sh"
    local database_dir="${PROJECT_ROOT}/database"

    cat > "${db_script}" << DBEOF
#!/bin/bash

# ClamAV 病毒库更新脚本
# 使用本地安装的 ClamAV 工具更新病毒库

set -e

SCRIPT_DIR="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")" && pwd)"
LOCAL_DIR="\${SCRIPT_DIR}/local"
ENV_SCRIPT="\${SCRIPT_DIR}/clamav_env.sh"
DATABASE_DIR="\${SCRIPT_DIR}/database"

# 加载环境变量
if [ -f "\${ENV_SCRIPT}" ]; then
    source "\${ENV_SCRIPT}"
else
    export CLAMAV_ROOT="\${LOCAL_DIR}"
    export PATH="\${LOCAL_DIR}/bin:\${LOCAL_DIR}/sbin:\${PATH}"
    export LD_LIBRARY_PATH="\${LOCAL_DIR}/lib:\${LD_LIBRARY_PATH}"
    export CLAMAV_DATABASE_DIR="\${DATABASE_DIR}"
    export CLAMAV_LOG_DIR="\${LOCAL_DIR}/var/log/clamav"
fi

# 配置路径
FRESHCLAM_CONF="\${LOCAL_DIR}/etc/freshclam.conf.sample"
LOG_DIR="\${LOCAL_DIR}/var/log/clamav"

# 创建必要目录
mkdir -p "\${DATABASE_DIR}"
mkdir -p "\${LOG_DIR}"

echo "========================================"
echo "  ClamAV 病毒库更新"
echo "========================================"
echo ""
echo "数据库目录: \${DATABASE_DIR}"
echo ""

# 使用 freshclam 更新病毒库
if [ -f "\${FRESHCLAM_CONF}" ]; then
    # 创建临时配置
    local temp_conf=\$(mktemp)
    cp "\${FRESHCLAM_CONF}" "\${temp_conf}"

    # 修改配置中的路径
    sed -i '' "s|^DatabaseDirectory.*|DatabaseDirectory \${DATABASE_DIR}|g" "\${temp_conf}"
    sed -i '' "s|^LogFile.*|LogFile \${LOG_DIR}/freshclam.log|g" "\${temp_conf}"
    sed -i '' "s|^DNSDatabaseInfo.*|DNSDatabaseInfo current.cvd.clamav.net|g" "\${temp_conf}"
    sed -i '' "s|^PrivateMirror.*|PrivateMirror database.clamav.net|g" "\${temp_conf}"

    echo "开始下载病毒库..."
    "\${LOCAL_DIR}/bin/freshclam" --config-file="\${temp_conf}"

    rm -f "\${temp_conf}"

    echo ""
    echo "✓ 病毒库更新完成"
    echo "病毒库位置: \${DATABASE_DIR}"
    ls -lh "\${DATABASE_DIR}"/*.cvd 2>/dev/null || ls -lh "\${DATABASE_DIR}"/*.cdiff 2>/dev/null || echo "等待下载完成..."
else
    echo "✗ 找不到 freshclam 配置文件"
    exit 1
fi
DBEOF

    chmod +x "${db_script}"
    log_info "✓ 病毒库更新脚本已创建: ${db_script}"
}

# 验证安装结果
verify_installation() {
    log_step "验证安装结果..."

    local errors=0

    echo ""
    echo "========================================"
    echo "  安装验证报告"
    echo "========================================"
    echo ""

    # 检查本地安装目录
    echo "1. 本地安装目录检查:"
    if [ -d "${LOCAL_INSTALL_DIR}" ]; then
        echo "   ✓ ${LOCAL_INSTALL_DIR} 存在"
        echo "   目录内容:"
        ls -la "${LOCAL_INSTALL_DIR}" | head -20
    else
        echo "   ✗ ${LOCAL_INSTALL_DIR} 不存在"
        ((errors++))
    fi
    echo ""

    # 检查可执行文件
    echo "2. 可执行文件检查:"
    local bin_files=("clamscan" "clamd" "freshclam" "sigtool")
    for bin in "${bin_files[@]}"; do
        if [ -f "${LOCAL_INSTALL_DIR}/bin/${bin}" ]; then
            echo "   ✓ ${bin} 已安装"
        elif [ -f "${LOCAL_INSTALL_DIR}/sbin/${bin}" ]; then
            echo "   ✓ ${bin} 已安装"
        else
            echo "   ✗ ${bin} 未找到"
            ((errors++))
        fi
    done
    echo ""

    # 检查库文件
    echo "3. 库文件检查:"
    if [ -d "${LOCAL_INSTALL_DIR}/lib" ]; then
        echo "   ✓ 库目录存在"
        echo "   库文件:"
        ls -lh "${LOCAL_INSTALL_DIR}/lib"/*.so* 2>/dev/null | head -10 || echo "   (仅静态库)"
    else
        echo "   ✗ 库目录不存在"
        ((errors++))
    fi
    echo ""

    # 检查配置文件
    echo "4. 配置文件检查:"
    if [ -d "${LOCAL_INSTALL_DIR}/etc" ]; then
        echo "   ✓ 配置目录存在"
        ls "${LOCAL_INSTALL_DIR}/etc/"*.conf* 2>/dev/null | head -5
    else
        echo "   ✗ 配置目录不存在"
        ((errors++))
    fi
    echo ""

    # 检查环境变量脚本
    echo "5. 环境变量脚本检查:"
    if [ -f "${PROJECT_ROOT}/clamav_env.sh" ]; then
        echo "   ✓ 环境变量脚本存在"
    else
        echo "   ✗ 环境变量脚本不存在"
        ((errors++))
    fi
    echo ""

    # 检查是否污染系统
    echo "6. 系统污染检查:"

    # 检查系统 PATH
    local in_system_path=false
    for path_dir in /usr/local/bin /usr/bin /usr/sbin /sbin; do
        if [ -f "${path_dir}/clamscan" ]; then
            echo "   ⚠ 发现系统安装的 clamscan: ${path_dir}/clamscan"
            in_system_path=true
        fi
    done

    if [ "$in_system_path" = false ]; then
        echo "   ✓ 系统 PATH 中未发现 ClamAV 可执行文件"
    fi

    # 检查系统库
    local in_system_lib=false
    for lib_dir in /usr/lib /usr/lib64 /lib /lib64; do
        if [ -f "${lib_dir}/libclamav"* ]; then
            echo "   ⚠ 发现系统安装的 libclamav: ${lib_dir}/libclamav*"
            in_system_lib=true
        fi
    done

    if [ "$in_system_lib" = false ]; then
        echo "   ✓ 系统库目录中未发现 libclamav"
    fi

    # 检查环境变量
    echo "   当前环境变量中的 ClamAV 路径:"
    echo "   - PATH: $(echo $PATH | tr ':' '\n' | grep -E '(clam|local)' || echo '无')"
    echo "   - LD_LIBRARY_PATH: $(echo $LD_LIBRARY_PATH | tr ':' '\n' | grep -E '(clam|local)' || echo '无')"

    echo ""

    # 验证可执行文件
    echo "7. 可执行文件功能验证:"
    if [ -f "${LOCAL_INSTALL_DIR}/bin/clamscan" ]; then
        # 测试 clamscan --version
        if "${LOCAL_INSTALL_DIR}/bin/clamscan" --version &> /dev/null; then
            echo "   ✓ clamscan 可正常运行"
            "${LOCAL_INSTALL_DIR}/bin/clamscan" --version | head -3
        else
            echo "   ✗ clamscan 无法正常运行"
            ((errors++))
        fi
    fi
    echo ""

    # 检查病毒库目录
    echo "8. 病毒库目录检查:"
    if [ -d "${DATABASE_DIR}" ]; then
        echo "   ✓ ${DATABASE_DIR} 存在"
    else
        echo "   ⚠ ${DATABASE_DIR} 不存在（可能尚未下载病毒库）"
        echo "   请运行: ${PROJECT_ROOT}/update_clamav_db.sh"
    fi
    echo ""

    # 总结
    echo "========================================"
    echo "  验证结果"
    echo "========================================"
    if [ $errors -eq 0 ]; then
        echo -e "${GREEN}✓ 安装验证通过${NC}"
        echo ""
        echo "使用说明:"
        echo "  1. 设置环境变量: source ${PROJECT_ROOT}/clamav_env.sh"
        echo "  2. 更新病毒库: ${PROJECT_ROOT}/update_clamav_db.sh"
        echo "  3. 使用扫描: ${LOCAL_INSTALL_DIR}/bin/clamscan <路径>"
        echo ""
        echo "病毒库路径: ${DATABASE_DIR}"
        echo "日志路径: ${LOCAL_INSTALL_DIR}/var/log/clamav"
        return 0
    else
        echo -e "${RED}✗ 发现 $errors 个问题${NC}"
        echo "请检查上述输出"
        return 1
    fi
}

# 主函数
main() {
    # 解析参数
    local do_clean=false
    local do_build_only=false
    local do_install_only=false
    local do_verify=false
    local do_full=false

    for arg in "$@"; do
        case $arg in
            --help)
                show_help
                exit 0
                ;;
            --clean)
                do_clean=true
                ;;
            --build-only)
                do_build_only=true
                ;;
            --install-only)
                do_install_only=true
                ;;
            --verify)
                do_verify=true
                ;;
            --full)
                do_full=true
                ;;
        esac
    done

    echo ""
    echo "========================================"
    echo "  ClamAV 本地化编译安装脚本"
    echo "========================================"
    echo ""
    echo "项目根目录: ${PROJECT_ROOT}"
    echo "源码目录: ${CLAMAV_SOURCE_DIR}"
    echo "安装目录: ${LOCAL_INSTALL_DIR}"
    echo "构建目录: ${BUILD_DIR}"
    echo ""

    # 完整流程
    if [ "$do_full" = true ]; then
        do_clean=true
        do_build_only=false
        do_install_only=false
        do_verify=true
    fi

    # 清理
    if [ "$do_clean" = true ]; then
        clean_build
    fi

    # 仅安装
    if [ "$do_install_only" = true ]; then
        install_clamav
        create_env_script
        create_db_update_script
        verify_installation
        exit $?
    fi

    # 检查依赖和源码
    check_dependencies
    check_source
    create_local_dirs

    # 配置和编译
    configure_cmake
    build_clamav

    # 仅编译
    if [ "$do_build_only" = true ]; then
        log_info "编译完成，安装步骤已跳过"
        log_info "可使用 --install-only 进行安装"
        exit 0
    fi

    # 安装
    install_clamav

    # 创建环境变量脚本
    create_env_script

    # 创建病毒库更新脚本
    create_db_update_script

    # 验证
    if [ "$do_verify" = true ]; then
        verify_installation
    fi
}

# 运行主函数
main "$@"
