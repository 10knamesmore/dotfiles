# 命令语法高亮（zsh-syntax-highlighting，冻结 vendor、不跟上游更新）。
# 必须在所有 zle widget 定义之后 source（它包裹已注册的 widget），故编号 90 最后。
source "${0:A:h}/vendor/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh"
