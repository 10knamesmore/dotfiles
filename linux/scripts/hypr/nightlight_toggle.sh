#!/usr/bin/env bash
set -euo pipefail

# 夜间模式 toggle：委托给 screen_effects.sh
exec "$(dirname "$0")/screen_effects.sh" toggle
