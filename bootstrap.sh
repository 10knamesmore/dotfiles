#!/usr/bin/env bash
# 新机一条命令：git clone <repo> ~/dotfiles && ~/dotfiles/bootstrap.sh
# 极薄引导：装 rustup（若缺）→ 编译 dots → 交棒给 dots bootstrap。
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"

# 裸系统没有 cc，cargo build（mlua vendored 编 C）会挂，先补编译前置。
# git 不在此装：能跑到这里说明仓库已到手；后续 paru 自举要的 git 由 pacman.txt 先行提供。
if ! command -v cc >/dev/null 2>&1; then
    if command -v pacman >/dev/null 2>&1; then
        echo "安装编译前置（base-devel）…"
        sudo pacman -S --needed --noconfirm base-devel
    elif command -v apt-get >/dev/null 2>&1; then
        echo "安装编译前置（build-essential）…"
        sudo apt-get update -qq && sudo apt-get install -y build-essential
    fi
fi

# cargo/rustup 镜像（rsproxy.cn）：首次编译发生在 dots sync 链接配置之前，
# 这里先落一份真实文件解决鸡生蛋；sync 时会备份并收编为软链（源：tree/home/.cargo/config.toml）。
# 已有配置（自带或软链）则不动；海外机器想直连可先自建空配置跳过。
if [ ! -e "$HOME/.cargo/config.toml" ]; then
    echo "写入 cargo 镜像配置（rsproxy.cn）…"
    mkdir -p "$HOME/.cargo"
    cat > "$HOME/.cargo/config.toml" << 'EOF'
[source.crates-io]
replace-with = 'rsproxy-sparse'

[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"

[registries.rsproxy]
index = "sparse+https://rsproxy.cn/index/"

[net]
git-fetch-with-cli = true
EOF
fi
export RUSTUP_DIST_SERVER="${RUSTUP_DIST_SERVER:-https://rsproxy.cn}"
export RUSTUP_UPDATE_ROOT="${RUSTUP_UPDATE_ROOT:-https://rsproxy.cn/rustup}"

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
