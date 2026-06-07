# zsh 核心行为：历史 / 目录导航 / 补全 / 键绑定

# --- 历史 ---
HISTFILE="$HOME/.zsh_history" # 历史落盘位置（zsh 默认不落盘，必须显式设）
HISTSIZE=50000                # 内存中保留的条数（启动时从文件读这么多）
SAVEHIST=1000000              # 文件中保留的条数（设得比 HISTSIZE 大 = 文件是全量档案）
setopt share_history          # 多终端实时共享：每条命令写完即落盘，其他终端的 ↑ 能立刻看到
# setopt hist_ignore_dups       # 与上一条相同的命令不重复入史
# setopt hist_ignore_all_dups   # 新命令与任何旧条目重复时，删掉旧的只留最新（历史里每条命令唯一）
setopt hist_reduce_blanks     # 入史前压掉命令里多余的空白
setopt extended_history       # 历史带时间戳和耗时（: 1700000000:5;cmd 格式，archaeology 用）
setopt hist_verify            # !! / !$ 等历史展开先回填到命令行让你确认，不直接执行
setopt hist_ignore_space      # 行首空格的命令不进历史（敲密钥/token 的逃生门）
setopt hist_no_store          # history 命令本身不进历史（翻历史不再看到一串 history）

# --- 目录导航 ---
setopt auto_cd           # 直接敲目录名等于 cd：`..`、`/etc`、`~/dotfiles` 都能直接走
setopt auto_pushd        # 每次 cd 自动把旧目录压进目录栈（配合下面的 1-9 别名跳回去）
setopt pushd_ignore_dups # 目录栈去重，免得 1-9 里出现一串相同路径
setopt pushd_minus       # 交换 +/- 含义：`cd -3` 表示「倒数第 3 个去过的」，比默认 + 顺手
DIRSTACKSIZE=9           # 目录栈深度上限，与 1-9 别名对齐
alias -- -='cd -'        # `-` 回上一个目录（-- 防止 - 被当成选项）
alias ..='cd ..'
alias ...='cd ../..'
alias ....='cd ../../..'
for _idx in {1..9}; do alias "$_idx"="cd +$_idx"; done # 1-9 = 跳到目录栈第 N 项（dirs -v 看栈）
unset _idx

# --- 杂项 ---
setopt interactive_comments # 粘贴含 # 注释的命令不报错
setopt numeric_glob_sort    # glob 结果数字按数值排序（1 2 10 而非 1 10 2）

# --- 补全：compinit + 匹配规则 + 菜单高亮（须在 uv 等工具注册补全之前跑）---
autoload -Uz compinit
mkdir -p "$HOME/.cache/zsh"
compinit -d "$HOME/.cache/zsh/zcompdump" # 初始化补全系统，dump 缓存挪出 $HOME 根目录
zmodload zsh/complist                    # 菜单选择（menuselect keymap）所需模块
# matcher-list 多档递进：前一档无匹配才落到下一档（档内规则空格分隔、同时生效）
#   m:{a-zA-Z}={A-Za-z}  大小写不敏感（双向）
#   l:|=* r:|=*          光标左右两侧各允许任意前后缀 = 子串匹配（conf → kitty.conf）
zstyle ':completion:*' matcher-list 'm:{a-zA-Z}={A-Za-z}' 'm:{a-zA-Z}={A-Za-z} l:|=* r:|=*'
zstyle ':completion:*' menu select                      # 候选 >1 时进入方向键可选的高亮菜单
zstyle ':completion:*' list-colors "${(s.:.)LS_COLORS}" # 候选文件按 LS_COLORS 着色
zstyle ':completion:*' group-name ''                    # 候选按类型分组（命令/别名/文件分开列）
zstyle ':completion:*:descriptions' format '%F{yellow}── %d ──%f' # 分组标题行样式
zstyle ':completion:*' use-cache true                   # 慢补全（docker/systemctl…）结果落盘缓存
zstyle ':completion:*' cache-path "$HOME/.cache/zsh/zcompcache"
zstyle ':completion:*' special-dirs true                # .. 和 . 也作为补全候选（cd ..<Tab> 成立）
zstyle ':completion:*' squeeze-slashes true             # 补全时 // 折叠为 /
setopt complete_in_word # 光标在词中间也能补全：右半截当后缀约束而非无视
setopt always_to_end    # 补全成功后光标跳到词尾（默认停在插入点）

# --- 键绑定 ---
# 显式 emacs 模式：zsh 见 $EDITOR 含 "vi"（nvim）会自动切 viins，曾导致 Ctrl-A/E 等全部失效
bindkey -e
# 释放 tty 流控遗产：默认 Ctrl-S 冻结终端输出（误按假死元凶）、Ctrl-Q 解冻；关掉后按键到达 zle
setopt no_flow_control
# 四键留白（按 2026-06 决定）：S/Q（原流控/搜索）、W/U（原删词/删整行）均解绑，待将来自定义
bindkey -r '^S' '^Q' '^W' '^U'
KEYTIMEOUT=1                      # 多字节键序列等待窗 400ms→10ms，Esc 类组合键不再迟滞
WORDCHARS=${WORDCHARS//\//}       # 「词」不再含 /：Ctrl-←/→（及将来的删词键）按路径段移动

# ↑↓ 前缀历史搜索：敲了 `git ` 再按 ↑ 只翻 git 开头的历史（光标留在行尾）
autoload -Uz up-line-or-beginning-search down-line-or-beginning-search
zle -N up-line-or-beginning-search
zle -N down-line-or-beginning-search
bindkey '^[[A' up-line-or-beginning-search   # ↑（CSI 序列）
bindkey '^[OA' up-line-or-beginning-search   # ↑（SS3 序列，部分终端/模式下发这组）
bindkey '^[[B' down-line-or-beginning-search # ↓
bindkey '^[OB' down-line-or-beginning-search
bindkey '^[[H' beginning-of-line             # Home
bindkey '^[[F' end-of-line                   # End
bindkey '^[[3~' delete-char                  # Delete
bindkey '^[[1;5C' forward-word               # Ctrl-→ 按词前跳
bindkey '^[[1;5D' backward-word              # Ctrl-← 按词后跳
# Shift-Tab 反向补全（OMZ key-bindings 原有，退役对照表漏收）：主 keymap 起步反向，menuselect 内后退
bindkey '^[[Z' reverse-menu-complete
bindkey -M menuselect '^[[Z' reverse-menu-complete
