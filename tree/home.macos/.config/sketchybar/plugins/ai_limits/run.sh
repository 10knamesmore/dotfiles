#!/bin/zsh
# 在用户登录 shell 环境中运行 AI 额度 item，使 launchd 启动的 SketchyBar 获得密钥和代理设置。
# Usage: run.sh <codex|kimi|opencode>
# 依赖：zsh，以及同目录的 item.sh
# 读取环境变量：HOME、NAME、SENDER；并从 ~/.zshrc 加载用户声明的凭据和网络环境
# 写入环境变量：仅影响当前子进程，不修改 launchd、凭据文件或 shell 配置

SCRIPT_DIR="${0:A:h}"

# Homebrew LaunchAgent 不读取交互式 shell 配置；显式加载一次，避免把 secret 展开进 script 属性。
source "$HOME/.zshrc" >/dev/null 2>&1
exec "$SCRIPT_DIR/item.sh" "$@"
