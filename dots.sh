#!/usr/bin/env bash
# 透传：把所有参数转发给 cargo run --release。
# 用法：./dots.sh <dots 子命令...>
#   ./dots.sh --help
#   ./dots.sh sync --dry-run
#   ./dots.sh status
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
# 显式告诉 dots 仓库根（迁移前还没 dots.lua，无法靠它定位）。
export DOTFILES_DIR="$HERE"
exec cargo run --release --quiet --manifest-path "$HERE/cli/Cargo.toml" -- "$@"
