# OMZ 退役补齐：历史 / 目录导航 / 补全 / 键绑定（对照 spec §7.3 表，逐项注明）

# --- #1 #2 历史（OMZ 原本设的 HISTFILE，不补则历史不落盘）---
HISTFILE="$HOME/.zsh_history"
HISTSIZE=50000
SAVEHIST=1000000
setopt share_history          # 多终端共享历史
setopt hist_ignore_dups hist_ignore_all_dups hist_reduce_blanks
setopt extended_history hist_verify inc_append_history

# --- #5 目录导航（.. ... - 1-9、auto_cd、auto_pushd）---
setopt auto_cd auto_pushd pushd_ignore_dups pushd_minus
DIRSTACKSIZE=9
alias -- -='cd -'
alias ..='cd ..'
alias ...='cd ../..'
alias ....='cd ../../..'
for _idx in {1..9}; do alias "$_idx"="cd +$_idx"; done
unset _idx

# --- #6 粘贴含 # 注释的命令不报错 ---
setopt interactive_comments

# --- #4 补全：compinit + 大小写不敏感 + 菜单高亮 + 缓存（须在 uv 补全前）---
autoload -Uz compinit
mkdir -p "$HOME/.cache/zsh"
compinit -d "$HOME/.cache/zsh/zcompdump"
zmodload zsh/complist
zstyle ':completion:*' matcher-list 'm:{a-zA-Z}={A-Za-z}'   # 大小写不敏感
zstyle ':completion:*' menu select                          # 菜单选择
zstyle ':completion:*' list-colors "${(s.:.)LS_COLORS}"     # 着色

# --- #3 键绑定：↑↓ 前缀历史搜索 + Home/End/Delete ---
autoload -Uz up-line-or-beginning-search down-line-or-beginning-search
zle -N up-line-or-beginning-search
zle -N down-line-or-beginning-search
bindkey '^[[A' up-line-or-beginning-search   # ↑
bindkey '^[OA' up-line-or-beginning-search
bindkey '^[[B' down-line-or-beginning-search # ↓
bindkey '^[OB' down-line-or-beginning-search
bindkey '^[[H' beginning-of-line             # Home
bindkey '^[[F' end-of-line                   # End
bindkey '^[[3~' delete-char                  # Delete
# Shift-Tab 反向补全（OMZ key-bindings 原有，退役对照表漏收）：主 keymap 起步反向，menuselect 内后退
bindkey '^[[Z' reverse-menu-complete
bindkey -M menuselect '^[[Z' reverse-menu-complete
