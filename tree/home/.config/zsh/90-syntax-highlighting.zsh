# 命令语法高亮使用仓库内固定的 fast-syntax-highlighting。
# 除命令存在性与引号括号闭合外，chroma 还提供 git/docker/ssh 等命令的选项级高亮。
# chroma 未覆盖的命令退回通用高亮，不报错。
# 同样必须在所有 zle widget 定义之后 source（它包裹已注册的 widget），故编号 90 最后。
_fsh_dir="${0:A:h}/vendor/fast-syntax-highlighting"

# fsh 在 secondary_theme.zsh 缺失时会通过网络获取主题；预建空文件可避免 shell 启动依赖网络。
# secondary theme 未启用，因此空文件不改变高亮行为。
_fsh_work="${XDG_CACHE_HOME:-$HOME/.cache}/fast-syntax-highlighting"
[[ -e $_fsh_work/secondary_theme.zsh ]] || { mkdir -p "$_fsh_work" && touch "$_fsh_work/secondary_theme.zsh"; }

source "$_fsh_dir/fast-syntax-highlighting.plugin.zsh"
unset _fsh_dir _fsh_work
