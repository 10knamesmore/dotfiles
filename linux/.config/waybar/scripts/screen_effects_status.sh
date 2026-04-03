#!/usr/bin/env bash
set -euo pipefail

# waybar 轮询脚本：调用 screen_effects.sh status 输出 JSON
# 脚本通过 generated/scripts/hypr/ 链接可用
script_dir="$(cd "$(dirname "$0")" && pwd)"
exec "$script_dir/../../scripts/hypr/screen_effects.sh" status 2>/dev/null \
  || echo '{"text":"☀ ?","tooltip":"screen_effects.sh not found"}'
