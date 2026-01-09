# 病毒查杀工具

本项目是一个基于 ClamAV 的病毒查杀工具，采用项目隔离方式安装，所有组件和病毒库都存储在项目本地目录中，不会影响系统全局环境。

## 项目简介

### 背景

在企业级安全防护场景中，需要一个可靠的病毒扫描引擎来检测恶意软件。本项目集成 ClamAV——业界成熟的开源病毒扫描引擎，并采用项目隔离的安装方式，确保：

- **环境隔离**：不修改系统配置，不污染全局环境
- **独立部署**：每个项目可以拥有独立的病毒库版本
- **灵活集成**：便于在不同环境中部署和迁移
- **权限安全**：遵循最小权限原则运行

### 主要特性

- **项目隔离安装**：所有组件安装在项目 `local/` 目录，不污染系统
- **病毒库本地化**：病毒库存储在 `database/` 目录，可独立管理
- **环境变量隔离**：通过 `clamav_env.sh` 脚本设置专用环境变量
- **零系统依赖**：不修改系统配置，不写入全局环境变量
- **自动化流程**：提供编译、安装、更新、扫描一条龙脚本

## 技术架构

### 核心组件

| 组件 | 说明 |
|------|------|
| ClamAV | 开源病毒扫描引擎，提供病毒检测能力 |
| freshclam | 病毒库更新工具 |
| clamscan | 命令行扫描工具 |
| clamd | 扫描守护进程（可选） |

### 病毒库说明

ClamAV 病毒库包含以下主要文件：

| 文件 | 大小 | 说明 |
|------|------|------|
| main.cvd | ~150MB | 主病毒库，包含主要病毒特征码 |
| daily.cvd | ~10-20MB | 每日更新，包含最新病毒特征码 |
| bytecode.cvd | ~500KB | 字节码库，包含高级检测规则 |

## 目录结构

```
virus-scanning-tool/
├── local/                  # ClamAV 本地安装目录
│   ├── bin/               # 可执行文件 (clamscan, freshclam 等)
│   ├── sbin/              # 系统可执行文件
│   ├── lib/               # 库文件
│   ├── include/           # 头文件
│   ├── etc/               # 配置文件模板
│   └── var/               # 运行时数据
│       ├── log/           # 日志目录
│       └── lib/           # 临时数据库目录
├── database/              # 病毒库存储目录
├── build_clamav_local.sh  # 编译安装脚本
├── clamav_env.sh          # 环境变量脚本
├── update_clamav_db.sh    # 病毒库更新脚本
├── run_scanner.sh         # 扫描运行脚本
└── README.md              # 本文档
```

## 快速开始

### 环境准备

确保系统已安装必要依赖：

**Ubuntu / Debian：**

```bash
sudo apt-get update
sudo apt-get install -y \
    cmake \
    build-essential \
    zlib1g-dev \
    libcurl4-openssl-dev \
    libxml2-dev \
    libssl-dev \
    check
```

**macOS：**

```bash
brew install \
    cmake \
    openssl \
    curl \
    libxml2 \
    zlib
```

### 步骤一：编译安装 ClamAV

```bash
./build_clamav_local.sh
```

脚本执行过程：

```
[1/6] 检查系统依赖...
[2/6] 创建本地目录结构...
[3/6] 配置 CMake...
[4/6] 编译 ClamAV（这可能需要几分钟）...
[5/6] 安装到 local/ 目录...
[6/6] 验证安装...
✓ ClamAV 安装完成！
```

### 步骤二：加载环境变量

```bash
source clamav_env.sh
```

设置的环境变量：

| 变量名 | 值 | 说明 |
|--------|-----|------|
| CLAMAV_ROOT | `{项目目录}/local` | ClamAV 安装根目录 |
| PATH | `{$CLAMAV_ROOT}/bin:{$CLAMAV_ROOT}/sbin:{$PATH}` | 添加可执行文件路径 |
| LD_LIBRARY_PATH | `{$CLAMAV_ROOT}/lib:{$LD_LIBRARY_PATH}` | 添加库文件路径 |
| PKG_CONFIG_PATH | `{$CLAMAV_ROOT}/lib/pkgconfig:{$PKG_CONFIG_PATH}` | 添加 pkg-config 路径 |
| CLAMAV_DATABASE_DIR | `{项目目录}/database` | 病毒库目录 |
| CLAMAV_LOG_DIR | `{$CLAMAV_ROOT}/var/log/clamav` | 日志目录 |

### 步骤三：下载病毒库

```bash
./update_clamav_db.sh
```

首次下载约需 200MB 流量，耗时取决于网络状况。

### 步骤四：执行扫描

```bash
# 扫描单个文件
./run_scanner.sh /path/to/file

# 递归扫描目录
./run_scanner.sh -r /path/to/directory

# 扫描并显示详细信息
./run_scanner.sh -v -r /path/to/directory
```

## 详细使用方法

### 编译安装脚本 (build_clamav_local.sh)

编译并安装 ClamAV 到项目本地目录。

**基本用法：**

```bash
./build_clamav_local.sh
```

**执行步骤：**

1. **检查系统依赖** - 验证 CMake、编译器等是否可用
2. **创建目录结构** - 创建 `local/` 及其子目录
3. **配置 CMake** - 生成编译配置，使用 `CMAKE_INSTALL_PREFIX` 指定本地路径
4. **编译** - 使用多核并行编译
5. **安装** - 将编译产物复制到 `local/` 目录
6. **验证** - 检查关键文件是否存在

**输出示例：**

```
========================================
  ClamAV 本地编译安装脚本
========================================

项目根目录: /path/to/project
安装目录: /path/to/project/local
构建目录: /path/to/project/build

[1/6] 检查系统依赖...
✓ 所有依赖已安装

[2/6] 创建本地目录结构...
✓ 目录创建完成

[3/6] 配置 CMake...
✓ CMake 配置完成

[4/6] 编译 ClamAV...
✓ 编译完成 (耗时: 5分32秒)

[5/6] 安装到 local/ 目录...
✓ 安装完成

[6/6] 验证安装...
✓ clamscan: /path/to/project/local/bin/clamscan
✓ freshclam: /path/to/project/local/bin/freshclam
✓ libclamav: /path/to/project/local/lib/libclamav.so

✓ ClamAV 安装完成！
```

### 环境变量脚本 (clamav_env.sh)

设置项目隔离的环境变量。此脚本仅修改当前 Shell 的环境变量，不会影响系统全局配置。

**使用方法：**

```bash
# 方法一：source 命令加载
source clamav_env.sh

# 方法二：直接在当前 Shell 中执行
. clamav_env.sh
```

**验证环境变量：**

```bash
# 检查 ClamAV 是否可用
clamscan --version

# 检查病毒库目录
echo $CLAMAV_DATABASE_DIR

# 检查库路径
echo $LD_LIBRARY_PATH
```

**在脚本中引用：**

```bash
#!/bin/bash
source clamav_env.sh

# 现在可以直接使用 clamscan
clamscan /path/to/scan
```

### 病毒库更新脚本 (update_clamav_db.sh)

下载或更新 ClamAV 病毒库。

**基本用法：**

```bash
./update_clamav_db.sh
```

**工作流程：**

1. 加载环境变量
2. 创建 `database/` 和日志目录
3. 生成临时 freshclam 配置文件
4. 执行病毒库下载
5. 清理临时文件

**输出示例：**

```
========================================
  ClamAV 病毒库更新
========================================

数据库目录: /path/to/project/database

开始下载病毒库...
ClamAV update process started at Thu Jan  9 10:00:00 2025
Downloading main.cvd [100%]
main.cvd updated (version: 268, sigs: 2547215)
Downloading daily.cvd [100%]
daily.cvd updated (version: 28687, sigs: 4237891)
Downloading bytecode.cvd [100%]
bytecode.cvd updated (version: 334, sigs: 70)

Database updated successfully!

✓ 病毒库更新完成
病毒库位置: /path/to/project/database
-rw-r--r--  1 user  staff   150M Jan  9 10:05 main.cvd
-rw-r--r--  1 user  staff    18M Jan  9 10:05 daily.cvd
-rw-r--r--  1 user  staff   500K Jan  9 10:05 bytecode.cvd
```

**手动更新：**

```bash
# 强制更新（即使没有新版本也重新下载）
./update_clamav_db.sh --force
```

### 扫描运行脚本 (run_scanner.sh)

执行病毒扫描，支持多种扫描模式和选项。

**基本语法：**

```bash
./run_scanner.sh [选项] <扫描路径>
```

**命令选项：**

| 短选项 | 长选项 | 说明 | 示例 |
|--------|--------|------|------|
| `-h` | `--help` | 显示帮助信息 | `./run_scanner.sh --help` |
| `-r` | `--recursive` | 递归扫描目录 | `./run_scanner.sh -r /path` |
| `-v` | `--verbose` | 显示详细信息 | `./run_scanner.sh -v /path` |
| 无 | `--move=<目录>` | 移动感染文件到指定目录 | `./run_scanner.sh --move=./infected /path` |
| 无 | `--copy=<目录>` | 复制感染文件到指定目录 | `./run_scanner.sh --copy=./quarantine /path` |
| 无 | `--exclude=<模式>` | 排除匹配的文件 | `./run_scanner.sh --exclude="*.tmp" /path` |
| 无 | `--exclude-dir=<模式>` | 排除匹配的目录 | `./run_scanner.sh --exclude-dir=".git" /path` |

**使用示例：**

```bash
# 扫描单个文件
./run_scanner.sh /path/to/file.txt

# 递归扫描整个目录
./run_scanner.sh -r /path/to/directory

# 扫描并显示详细信息
./run_scanner.sh -v -r /path/to/directory

# 排除特定文件类型
./run_scanner.sh -r --exclude="*.log" --exclude="*.tmp" /path

# 排除特定目录
./run_scanner.sh -r --exclude-dir=".git" --exclude-dir="node_modules" /path

# 扫描并隔离感染文件
./run_scanner.sh -r --move=./infected /path

# 组合使用多个选项
./run_scanner.sh -v -r --exclude="*.log" --exclude-dir=".git" /path
```

**扫描输出示例：**

```
========================================
  ClamAV 病毒扫描
========================================

扫描路径: /path/to/scan
病毒库: /path/to/project/database

/path/to/scan/clean.txt: OK
/path/to/scan/eicar.com: Eicar-Test-File FOUND
/path/to/scan/subdir/test.exe: OK

----------- SCAN SUMMARY -----------
Known viruses: 6781234
Engine version: 1.0.3
Scanned directories: 3
Scanned files: 15
Infected files: 1
Data scanned: 1.50 MB
Data read: 2.00 MB (ratio 0.75)
Time: 0.123 sec (0 m 0 s)
```

**扫描结果说明：**

| 字段 | 说明 |
|------|------|
| Known viruses | 病毒库中的病毒特征码数量 |
| Engine version | ClamAV 引擎版本 |
| Scanned directories | 扫描的目录数量 |
| Scanned files | 扫描的文件数量 |
| Infected files | 发现感染的文件数量 |
| Data scanned | 扫描的数据量 |
| Data read | 读取的数据量 |
| Time | 扫描耗时 |

## 高级配置

### 自定义病毒库位置

默认情况下，病毒库存储在 `database/` 目录。如需修改，可在 `update_clamav_db.sh` 中调整：

```bash
# 修改此行
DATABASE_DIR="${SCRIPT_DIR}/your-custom-directory"
```

### 自定义日志位置

日志默认保存在 `local/var/log/clamav/` 目录。如需修改：

```bash
# 在 update_clamav_db.sh 中修改
LOG_DIR="/your/custom/log/path"
```

### 批量扫描脚本示例

创建 `batch_scan.sh` 进行定时扫描：

```bash
#!/bin/bash

# 批量扫描脚本
# 用法: ./batch_scan.sh /path/to/scan1 /path/to/scan2 ...

source clamav_env.sh

REPORT_DIR="./reports"
mkdir -p "$REPORT_DIR"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

for path in "$@"; do
    echo "扫描: $path"
    ./run_scanner.sh -r "$path" > "$REPORT_DIR/scan_${TIMESTAMP}_$(basename "$path").txt" 2>&1
done

echo "扫描完成，报告保存在: $REPORT_DIR"
```

### 定时更新病毒库

使用 cron 定时更新病毒库：

```bash
# 编辑 crontab
crontab -e

# 添加每日凌晨 3 点自动更新
0 3 * * * cd /path/to/project && ./update_clamav_db.sh >> /path/to/project/logs/update.log 2>&1
```

## 隔离性验证

本项目实现了完整的项目隔离，确保不会影响系统环境：

| 验证项 | 实现方式 | 状态 |
|--------|----------|------|
| 安装路径隔离 | 使用 `CMAKE_INSTALL_PREFIX` 指定 `local/` 目录 | ✓ |
| 环境变量隔离 | 通过 `source` 加载，仅当前 Shell 生效 | ✓ |
| 库路径隔离 | 使用 `LD_LIBRARY_PATH` 指定 `local/lib` | ✓ |
| 病毒库隔离 | 存储在 `database/` 目录 | ✓ |
| 配置隔离 | 使用本地临时配置文件 | ✓ |
| 日志隔离 | 保存在 `local/var/log/` 目录 | ✓ |

**验证测试：**

```bash
# 1. 加载环境变量前
echo $PATH | grep -q "local/bin" && echo "已污染" || echo "干净"

# 2. 加载环境变量后
source clamav_env.sh
echo $PATH | grep -q "local/bin" && echo "已隔离" || echo "错误"

# 3. 新开终端验证
# (新终端不会自动加载环境变量，保持干净)
```

## 常见问题

### 编译相关

**Q：CMake 配置失败，提示找不到编译器**

A：请确保已安装 build-essential 或 Xcode 命令行工具：

```bash
# Ubuntu/Debian
sudo apt-get install build-essential

# macOS
xcode-select --install
```

**Q：编译过程中内存不足**

A：减少并行编译的线程数：

```bash
# 在 build_clamav_local.sh 中修改
make -j2  # 使用 2 个并行作业
```

**Q：编译时间过长**

A：首次编译需要较长时间（约 10-30 分钟），取决于硬件性能。可以使用以下命令查看进度：

```bash
# 在 build_clamav_local.sh 中临时添加
make VERBOSE=1
```

### 病毒库相关

**Q：病毒库下载失败**

A：可能的原因：
1. 网络连接问题
2. 防火墙阻止访问 ClamAV 服务器
3. DNS 解析问题

解决方案：

```bash
# 检查网络
ping database.clamav.net

# 手动下载
cd database
curl -O https://database.clamav.net/main.cvd
curl -O https://database.clamav.net/daily.cvd
curl -O https://database.clamav.net/bytecode.cvd
```

**Q：病毒库更新后扫描找不到病毒**

A：请检查病毒库目录是否正确：

```bash
source clamav_env.sh
ls -lh $CLAMAV_DATABASE_DIR
```

**Q：病毒库文件损坏**

A：删除损坏的文件后重新下载：

```bash
rm -rf database/*
./update_clamav_db.sh
```

### 扫描相关

**Q：扫描速度慢**

A：可以尝试以下优化：
1. 使用 `--exclude` 排除大文件或不需要扫描的文件
2. 使用 `--exclude-dir` 排除不需要扫描的目录
3. 限制最大扫描文件大小

```bash
# 示例：排除常见的大文件目录
./run_scanner.sh -r --exclude="*.zip" --exclude="*.gz" --exclude-dir="node_modules" /path
```

**Q：clamscan: error while loading shared libraries**

A：库路径未正确设置，请确保已加载环境变量：

```bash
source clamav_env.sh
```

**Q：扫描结果显示 "unknown option"**

A：使用的是系统自带的 clamscan，而非项目本地版本：

```bash
# 确认使用的是本地版本
which clamscan
# 应显示: /path/to/project/local/bin/clamscan

# 如不正确，请重新加载环境变量
source clamav_env.sh
```

## 注意事项

1. **环境变量时效性**：每次新开终端都需要重新执行 `source clamav_env.sh`

2. **磁盘空间**：病毒库首次下载需要约 200MB 空间

3. **定期更新**：建议每周执行 `./update_clamav_db.sh` 更新病毒库

4. **编译时间**：首次编译可能需要 10-30 分钟

5. **权限要求**：确保对 `local/` 和 `database/` 目录有读写权限

6. **系统兼容性**：本项目在 macOS 和 Linux 上测试通过

## 相关资源

- [ClamAV 官方网站](https://www.clamav.net/)
- [ClamAV 文档](https://docs.clamav.net/)
- [ClamAV 病毒库](https://database.clamav.net/)

## 许可证

本项目使用 MIT 许可证。

ClamAV 使用 LGPL-2.1 许可证，详情请参阅 clamav/ 目录下的 LICENSE 文件。
