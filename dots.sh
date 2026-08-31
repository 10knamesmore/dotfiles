#!/usr/bin/env bash
# 通过 release profile 运行工作区中的 dots binary，并转发全部参数。
# Usage: ./dots.sh <dots 子命令...>
#   ./dots.sh --help
#   ./dots.sh install
#   ./dots.sh sync --dry-run
#   ./dots.sh status
# 依赖：cargo
# 设置 DOTFILES_DIR，供 dots 和其子进程定位当前仓库。
# 参数或 cargo 执行失败时立即退出，避免返回伪成功。
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
export DOTFILES_DIR="$HERE"
# --bin dots：workspace 里还有 agent-hook，不指名会歧义报错。
exec cargo run --release --quiet --bin dots --manifest-path "$HERE/cli/Cargo.toml" -- "$@"
