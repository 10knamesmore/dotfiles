# OMZ 退役内联：cd-ls / allclear / copypath / copyfile / 补全等待点
# （取代原 omz_custom 插件与 OMZ 内置 copypath/copyfile）

autoload -Uz add-zsh-hook

# cd 后自动 ls（原 cd-ls 插件）
_dots_cdls() { [[ -o interactive ]] && eval "${CD_LS_COMMAND:-ls}" }
add-zsh-hook chpwd _dots_cdls

# cd 回 $HOME 自动 clear（原 zsh-allclear 插件）
_dots_allclear() { [[ $PWD == $HOME ]] && clear }
add-zsh-hook chpwd _dots_allclear

# 复制当前/指定路径到剪贴板（原 OMZ copypath）
copypath() {
    local target="${1:-.}"
    realpath -- "$target" | tr -d '\n' | wl-copy
}

# 复制文件内容到剪贴板（原 OMZ copyfile）
copyfile() {
    [[ -f "$1" ]] && wl-copy < "$1"
}

# 补全计算慢时显示红点（原 COMPLETION_WAITING_DOTS）
_dots_complete_waiting() {
    print -Pn "%F{red}…%f"
    zle expand-or-complete
    zle redisplay
}
zle -N _dots_complete_waiting
bindkey '^I' _dots_complete_waiting
