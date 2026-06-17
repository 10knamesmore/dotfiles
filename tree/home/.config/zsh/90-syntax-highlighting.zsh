# 命令语法高亮（fast-syntax-highlighting v1.55，冻结 vendor、不跟上游更新）。
# 取代原 zsh-syntax-highlighting：除「命令存不存在 / 引号括号闭合」外，→chroma/ 还做
# 命令级语法感知（git/docker/ssh 等子命令、选项级高亮）。chroma 覆盖不全是已知的，
# 未收录的命令退回通用高亮，不报错。
# 同样必须在所有 zle widget 定义之后 source（它包裹已注册的 widget），故编号 90 最后。
_fsh_dir="${0:A:h}/vendor/fast-syntax-highlighting"

# fsh 首次加载若 secondary_theme.zsh 不存在，会 curl 去 GitHub 拉 share/free_theme.zsh——
# raw.githubusercontent 连不通时（新 shell 默认无代理）TCP 超时会卡住首个 prompt。预建一个
# 空占位让它跳过（插件判定用 -e，只看在不在）。注意：该 free_theme.zsh 在本冻结版上游就是
# 空文件，secondary 主题本是 no-op，建空不丢任何功能——别误以为要去填内容。
_fsh_work="${XDG_CACHE_HOME:-$HOME/.cache}/fast-syntax-highlighting"
[[ -e $_fsh_work/secondary_theme.zsh ]] || { mkdir -p "$_fsh_work" && touch "$_fsh_work/secondary_theme.zsh"; }

source "$_fsh_dir/fast-syntax-highlighting.plugin.zsh"
unset _fsh_dir _fsh_work
