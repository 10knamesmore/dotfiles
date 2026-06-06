#!/usr/bin/env bash
# 新机一条命令：git clone <repo> ~/dotfiles && ~/dotfiles/bootstrap.sh
# 极薄引导：装 rustup（若缺）→ 编译 dots → 交棒给 dots bootstrap。
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"

if ! command -v cargo >/dev/null 2>&1; then
    echo "未检测到 cargo，安装 rustup（minimal）…"
    curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi

echo "编译 dots（首次含 mlua vendored，需几分钟）…"
cargo build --release --manifest-path "$HERE/cli/Cargo.toml"

DOTS="$HERE/cli/target/release/dots"
echo "交棒给 dots bootstrap…"
exec "$DOTS" bootstrap
