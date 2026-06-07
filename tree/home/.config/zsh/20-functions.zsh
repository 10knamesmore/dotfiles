# OMZ 退役内联：cd-ls / allclear / copypath / copyfile / 补全等待点
# （取代原 omz_custom 插件与 OMZ 内置 copypath/copyfile）

autoload -Uz add-zsh-hook

# cd 后自动 ls（原 cd-ls 插件）
_dots_cdls() { [[ -o interactive ]] && eval "${CD_LS_COMMAND:-ls}" }
add-zsh-hook chpwd _dots_cdls

# cd 回 $HOME 自动 clear（原 zsh-allclear 插件）
_dots_allclear() { [[ $PWD == $HOME ]] && clear }
add-zsh-hook chpwd _dots_allclear

# 剪贴板写入分派（补回 OMZ clipcopy 抽象——内联时曾被硬编码成 wl-copy）
_dots_clipcopy() {
    if command -v pbcopy >/dev/null; then pbcopy          # macOS
    elif [[ -n $WAYLAND_DISPLAY ]] && command -v wl-copy >/dev/null; then wl-copy
    elif command -v xclip >/dev/null; then xclip -selection clipboard
    else print -u2 "clipcopy: 没有可用的剪贴板工具"; return 1
    fi
}

# 复制当前/指定路径到剪贴板（原 OMZ copypath）
copypath() {
    local target="${1:-.}"
    realpath -- "$target" | tr -d '\n' | _dots_clipcopy
}

# 复制文件内容到剪贴板（原 OMZ copyfile）
copyfile() {
    [[ -f "$1" ]] && _dots_clipcopy < "$1"
}

# 代理开关：proxy [on|off|status]，裸调等于 on（.zshrc 启动时调）。
# URL 可被 DOTS_PROXY_URL 覆盖（per-host 想换端口时 export 它即可，不用改这里）。
# 大小写都设：CLI 工具多读小写，requests/httpx 等会读大写。
proxy() {
    local url="${DOTS_PROXY_URL:-http://127.0.0.1:7897}"
    case "${1:-on}" in
        on)
            export http_proxy="$url" https_proxy="$url" all_proxy="$url"
            export HTTP_PROXY="$url" HTTPS_PROXY="$url" ALL_PROXY="$url"
            export no_proxy="127.0.0.1,localhost,::1,xz07" NO_PROXY="127.0.0.1,localhost,::1,xz07"
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

# 补全计算慢时显示红点（原 COMPLETION_WAITING_DOTS）
# 2026-06 起死代码：Tab 已被 25-fzf-tab.zsh 接管（source 顺序在后，^I 绑定被覆盖）。
# 若将来弃用 fzf-tab 可解开注释恢复。
# _dots_complete_waiting() {
#     print -Pn "%F{red}…%f"
#     zle expand-or-complete
#     zle redisplay
# }
# zle -N _dots_complete_waiting
# bindkey '^I' _dots_complete_waiting
