# fzf 键绑定 + fzf-tab 补全菜单（v1.3.0 冻结 vendor、不跟上游更新）。
# 顺序敏感：fzf --zsh 会绑 Tab（** 触发的模糊补全），fzf-tab 必须在它之后 source
# 才能接管 Tab；而 fzf-tab 又要求在 compinit（10-options）之后、autosuggestions
# （30）之前——所以 fzf 全家都住在这个 25 号缝里。
# 体验：整行高亮（含描述列）、续敲即时过滤且匹配字符高亮、Esc 干净退回原命令行、
#       `/` 连续补全深路径、< > 切换候选分组。
# --- fzf 本体配置（env 旋钮）---
# 组合规则：每个入口实际生效 = FZF_DEFAULT_OPTS + 各自 FZF_*_OPTS 追加（同名项后者覆盖）。
# 界面部件地图：
#   ╭─────────────────────────────╮ ← border
#   │ > git st█        ← prompt(>) + 查询
#   │   270/270 (0)    ← info 计数器 / spinner
#   │ ▌ git status     ← 光标行：pointer(▌) + fg+ 文字 + bg+ 底色
#   │   git stash      ←   匹配字符 st = hl 色
#   │ ▍ git stage      ← 多选已勾选行：marker(▍) + selected-bg 底色
#   ╰─────────────────────────────╯

# 数据源换 fd：尊重 .gitignore（node_modules/target 不刷屏）、含隐藏文件、跟随符号链接、
# 比默认的 find 快一个量级
export FZF_DEFAULT_COMMAND='fd --hidden --follow --exclude .git'

# 配色 = Catppuccin Mocha 官方调色板。逐键：
#   bg+          光标行底色 = surface0（「整行高亮」就是它）
#   bg           窗口背景；-1 = 不发背景色码，继承终端背景——保住 kitty 透明；
#                写死色值会把格子涂成不透明色块
#   spinner      数据流入时的加载转圈 = rosewater
#   hl / hl+     匹配字符（普通行 / 光标行），同值保证选中不变脸 = peach + 下划线斜体；
#                属性维度与 fzf-tab 彩虹组标题正交（组标题无下划线），永不撞色
#   fg / fg+     候选文字（普通行 / 光标行）= text，同色——选中态靠 bg+ 区分，文字可读性不降
#   header       --header 固定说明行 = red（平时少见，备用）
#   info         计数器行 = mauve
#   pointer      光标行最左指针符 = rosewater
#   prompt       查询输入前的 > = mauve（与 info 同族）
#   marker       多选模式已勾选行的行首标记 = lavender
#   selected-bg  多选已勾选行底色 = surface1；行底色三层级：普通(透明) < bg+ < selected-bg
# 从 --style=full 手工摘取的子项（读 options.go applyPreset 得到的等价展开），
# 只要输入框分离的观感、不要 header/preview 的厚 chrome（会挤掉 fzf-tab 候选行）：
#   --input-border=rounded  输入区独立围框
#   --info=inline-right     计数器贴输入框右端（inline 会挤在光标旁）
#   --highlight-line        选中行高亮延伸到整行宽度
export FZF_DEFAULT_OPTS="--layout=reverse --border=rounded --info=inline-right --highlight-line --popup --input-border=rounded \
--color=bg+:#313244,bg:-1,spinner:#f5e0dc,hl:#fab387:underline:italic \
--color=fg:#cdd6f4,header:#f38ba8,info:#cba6f7,pointer:#f5e0dc \
--color=marker:#b4befe,fg+:#cdd6f4,prompt:#cba6f7,hl+:#fab387:underline:italic \
--color=selected-bg:#45475a"

# Ctrl-T（文件路径插入）数据源：FZF_DEFAULT_COMMAND 只管「直接跑 fzf」的场景，
# Ctrl-T 组件读的是自己这个变量，两者不自动共享——必须显式抄一份
export FZF_CTRL_T_COMMAND="$FZF_DEFAULT_COMMAND"

# Ctrl-T 预览：{} = 当前候选路径（fzf 自动加引号防空格）。
# 文件 → bat 语法高亮：--style=numbers 只留行号（预览窗已有边框，去掉 bat 的装饰）、
#   --line-range=:200 只读前 200 行——预览命令每次光标移动都重跑，限幅是手感硬约束；
# 目录 → bat 报错被 2>/dev/null 吞掉，|| 回退 eza 两层树。一个表达式完成文件/目录分流
export FZF_CTRL_T_OPTS="--preview 'bat --color=always --style=numbers --line-range=:200 {} 2>/dev/null || eza -T --level=2 --color=always --icons {}'"

# Alt-C（模糊 cd）数据源：--type d 只列目录（cd 给文件候选没意义）、含隐藏目录
# （.config 能跳）、排除 .git 内部噪音
export FZF_ALT_C_COMMAND='fd --type d --hidden --exclude .git'

# Alt-C 预览：候选目录两层树——跳之前确认，满地的 src/build 同名目录靠内容一眼分辨
export FZF_ALT_C_OPTS="--preview 'eza -T --level=2 --color=always --icons {}'"

# Ctrl-R（历史搜索）：预览内容 = 完整命令本身（列表里超长命令被截断，预览窗看全）；
# 预览窗在下方、高 3 行、默认隐藏（90% 的搜索用不到），Ctrl-/ 现场开关
export FZF_CTRL_R_OPTS="--preview 'echo {}' --preview-window=down:3:hidden:wrap --bind 'ctrl-/:toggle-preview'"

command -v fzf >/dev/null && source <(fzf --zsh)   # Ctrl-R 模糊历史 / Ctrl-T 文件 / Alt-C cd
source "${0:A:h}/vendor/fzf-tab/fzf-tab.zsh"

zstyle ':fzf-tab:*' switch-group '<' '>'           # 默认 F1/F2 够不着，< > 顺手
zstyle ':fzf-tab:*' use-fzf-default-opts yes       # 补全菜单跟随上面的 Mocha 配色
zstyle ':fzf-tab:*' prefix ''                      # 去掉候选左侧的 · 前缀（与 . 易混）
# 高度补偿帐本：外框 2 行 + input-border 2 行 + 基础 chrome 2 行 = 6。
# fzf-tab 按「候选数 + fzf-pad」算菜单高度，对 opts 里的边框开销是盲的，pad 替它报销。
# 注意：fzf-tab 首次补全会把算出的高度钉进 FZF_TMUX_HEIGHT（:= 赋值），同 shell 后续
# 菜单全部复用——调试高度问题必须开新 shell（或 unset FZF_TMUX_HEIGHT）。
zstyle ':fzf-tab:*' fzf-pad 6
zstyle ':fzf-tab:*' fzf-min-height 15   # 高度下限：首次钉死也至少 15 行，不会被小菜单拖矮
