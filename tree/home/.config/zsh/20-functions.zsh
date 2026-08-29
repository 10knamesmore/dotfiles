# 交互辅助函数：目录 hook、剪贴板、代理和临时 scratch。

autoload -Uz add-zsh-hook

# chpwd hook：切换目录后自动 ls。
_dots_cdls() { [[ -o interactive ]] && eval "${CD_LS_COMMAND:-ls}" }
add-zsh-hook chpwd _dots_cdls

# chpwd hook：回到 $HOME 时清屏。
_dots_allclear() { [[ $PWD == $HOME ]] && clear }
add-zsh-hook chpwd _dots_allclear

# 按 macOS、Wayland、X11 顺序选择可用的剪贴板写入命令。
_dots_clipcopy() {
    if command -v pbcopy >/dev/null; then pbcopy          # macOS
    elif [[ -n $WAYLAND_DISPLAY ]] && command -v wl-copy >/dev/null; then wl-copy
    elif command -v xclip >/dev/null; then xclip -selection clipboard
    else print -u2 "clipcopy: 没有可用的剪贴板工具"; return 1
    fi
}

# 复制当前或指定路径到剪贴板。
copypath() {
    local target="${1:-.}"
    realpath -- "$target" | tr -d '\n' | _dots_clipcopy
}

# 复制文件内容到剪贴板。
copyfile() {
    [[ -f "$1" ]] && _dots_clipcopy < "$1"
}

# 代理开关：proxy [on|off|status]，裸调等于 on（.zshrc 启动时调）。
# URL 可被 DOTS_PROXY_URL 覆盖。
# 大小写都设：CLI 工具多读小写，requests/httpx 等会读大写。
proxy() {
    local url="${DOTS_PROXY_URL:-http://127.0.0.1:7897}"
    case "${1:-on}" in
        on)
            export http_proxy="$url" https_proxy="$url" all_proxy="$url"
            export HTTP_PROXY="$url" HTTPS_PROXY="$url" ALL_PROXY="$url"
            export no_proxy="127.0.0.1,localhost,::1,xz07,non-convex.tech,.non-convex.tech" NO_PROXY="127.0.0.1,localhost,::1,xz07,non-convex.tech,.non-convex.tech"
            ;;
        off)
            unset http_proxy https_proxy all_proxy ftp_proxy
            unset HTTP_PROXY HTTPS_PROXY ALL_PROXY FTP_PROXY
            print "proxy off"
            ;;
        status)
            print -- "${http_proxy:-proxy off}"
            ;;
        *)
            print -u2 "用法: proxy [on|off|status]"
            return 1
            ;;
    esac
}

# 即用即丢临时文件：/tmp（tmpfs）重启即清，不保存则连文件都不留。
# 默认 .md，`sc py` / `sc json` 换扩展名（nvim 高亮跟扩展名走）。
# 持久便签走 nvim 内 snacks scratch（`.`），与这里互不相干。
sc() { nvim "/tmp/scratch-$$-$(date +%H%M%S).${1:-md}" }

# Ctrl-Q 直达 sc（留白键启用，Q=Quick note；10-options 已 no_flow_control + 解绑）。
# 回填 BUFFER 再 accept-line 而非直接调函数：进 shell 历史，prompt 流程正常走
_dots_scratch_widget() { BUFFER="sc"; zle accept-line }
zle -N _dots_scratch_widget
bindkey '^Q' _dots_scratch_widget
