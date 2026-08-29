#!/usr/bin/env bash
# 检查编译依赖，构建 dots，再收敛仓库声明。
# Usage: ./bootstrap.sh
# 依赖：cc、cargo
# 环境变量：无
# 任一步失败立即退出，避免用不完整的 binary 继续修改本机配置。
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"

if ! command -v cc >/dev/null 2>&1; then
    echo "缺少 C 编译器 cc；请先安装后重试。" >&2
    exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "缺少 Cargo；请先安装 Rust 工具链后重试。" >&2
    exit 1
fi

echo "编译 dots…"
cargo build --release --manifest-path "$HERE/cli/Cargo.toml"

DOTS="$HERE/cli/target/release/dots"
echo "运行 dots sync…"
exec "$DOTS" sync
