# ================================
# 🤖 Agent 守卫
# ================================
# Claude Code 等 agent 会把交互 shell 的 alias/函数快照进自己的 Bash 工具：
# rm -i 在无 TTY 下静默失败（exit 0 但没删）、sed -E / grep→rg 偷改语义、
# tree() 吃不了 -L。agent 环境（CLAUDECODE=1）直接不加载本文件，模型用原生命令。
# 注意：不能换成 [[ -o interactive ]]——CC 捕快照用的就是交互式 zsh（否则
# .zshrc 不会被 source），交互检查拦不住它；CLAUDECODE 是唯一区分信号。
[[ -n "$CLAUDECODE" ]] && return

# ================================
# 🧰 通用命令替代 & 默认行为增强
# ================================
alias activate="source ./.venv/bin/activate"
alias chmod='chmod -v'
alias chown='chown -v'
alias cp='cp -v'
alias df="df -hT"
alias free="free -hw --si"
alias ln='ln -v'
alias mkdir='mkdir -pv'
alias mount='mount -v'
alias mv='mv -v'
alias remove='rm -Iv' # 更强交互模式
alias rm='rm -iv'     # 删除确认
alias sed="sed -E"
alias sudoed='sudo nvim'
alias umount='umount -v'
alias venv="python -m venv .venv"
alias bim='vim'
alias vim='nvim -V1'
alias grep="rg"
alias rg="rg -i"

# ================================
# 🧠  系统信息
# ================================
alias how='tldr'
alias lsblk='lsblk -o NAME,FSTYPE,UUID,MOUNTPOINT,FSAVAIL'
alias showPath="echo \$PATH | sed 's/:/\\n/g'"

# ================================
# 🗂️ 目录/配置文件 快捷打开
# ================================
alias zsh_config="nvim ~/.zshrc"
alias nvim_config="nvim ~/.config/nvim/init.lua"

# ================================
# 🖥️ GUI 程序快捷启动（后台）
# ================================
neo() {
  neovide --frame none </dev/null >/dev/null 2>&1 & disown
}

# ================================
# 📁 文件/目录显示增强（使用 eza）
# ================================
alias ls="eza --icons --git --group-directories-first"
alias la='eza -lahHiM --icons --git --group-directories-first'
alias l='eza -lh --icons --git --group-directories-first'
alias ll='eza -lh --icons --git --group-directories-first'
tree() {
    local level=${1:-2}
    eza -T --icons --git --level="${level}"
}

# ================================
# 🧪 Git 快捷命令
# ================================
alias lg="lazygit"
alias g="git"
alias ga='git add'
alias gaa='git add -A'
alias gc='git commit -m'
alias gst='git status'
alias gco='git checkout'
alias gs="git switch"
alias gf='git fetch'
alias gp='git push'
alias gl='git pull'
alias gb='git branch'
alias glo='git log --oneline --graph --decorate'
alias gloa='git log --oneline --graph --decorate --all'
alias gla="git log --graph --decorate --all"
gline() {
    git log --since=midnight --author="$(git config user.name)" --pretty=tformat: --numstat |
        awk '{ added += $1; removed += $2 } END { print "Added:", added, "Removed:", removed }'
}

# yazi 退出时 cd 到浏览位置
function y() {
    local tmp="$(mktemp -t "yazi-cwd.XXXXXX")" cwd
    yazi "$@" --cwd-file="$tmp"
    if cwd="$(command cat -- "$tmp")" && [ -n "$cwd" ] && [ "$cwd" != "$PWD" ]; then
        builtin cd -- "$cwd"
    fi
    \rm -f -- "$tmp"
}

# ================================
# 🪟 作业控制 / Zellij
# ================================
alias f="fg"
alias b="bg"
alias j="jobs"
alias jobs="jobs -l"
alias zj="zellij"
alias zjl="zellij list-sessions"
alias zja="zellij attach"

# ================================
# AI
# ================================
alias cc="claude"
