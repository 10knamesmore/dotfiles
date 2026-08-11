# Shell 文档注释规范（Bash / Zsh）

Shell 没有语言级文档系统，注释就是唯一的「文档」。本规范约定 dotfiles 仓库里
`scripts/`、安装脚本、`tree/home/.config/zsh/` 等处的注释写法，目标是：脚本头能
读懂职责与依赖，函数能读懂契约，影响全局的语句能读懂理由。

核心原则：

* 全中文撰写，技术术语（`PATH`、`trap`、`subshell`、`exit code` 等）保留英文。
* 注释解释「做什么 / 为什么」，不逐字翻译命令。自明语句不写注释。
* 对外复用的函数库走「完整档」（shdoc 风格），脚本内部辅助函数走「日常档」。
* 一切影响全局行为的语句（`set`、`trap`、`IFS`、`export`）必须注明理由。

---

## 1. 文件头注释

每个可执行脚本顶部，紧跟 shebang 之后，写一段文件头注释。必须覆盖：

* **职责**：这个脚本是干什么的（一句话）。
* **Usage**：怎么调用，含参数与典型示例。
* **依赖**：用到的外部命令（非 coreutils 的、可能缺失的优先列）。
* **环境变量**：读哪些、写哪些，及其语义/默认值。

```bash
#!/usr/bin/env bash
#
# 把 tree/ 下的配置软链接到 $HOME，幂等，可反复执行。
#
# Usage:
#   sync.sh [--dry-run] [--host <name>]
#   --dry-run   只打印将要执行的链接操作，不落盘
#   --host      指定 per-host 资产，缺省取 hostname
#
# 依赖：fd, ln, realpath, age（解密 secrets 时才需要）
# 读取环境变量：
#   DOTFILES_DIR  仓库根，缺省由脚本自身路径推导
#   DOTS_DRY_RUN  非空时等价于 --dry-run
# 写入环境变量：无（仅写 ~/.config/dots/env.zsh）
```

只有寥寥几行的小工具，文件头可以压缩，但**职责 + Usage 两行不能省**：

```bash
#!/usr/bin/env bash
# 输出默认网卡的网络状态 JSON，供 QuickShell NetworkModule 消费。
# Usage: network_status.sh   （无参数，stdout 输出 {"icon","value","tooltip","class"}）
# 依赖：ip, nmcli, awk
```

---

## 2. 函数注释：两档制

### 何时用哪一档

* **完整档（shdoc 风格）**：对外复用的函数库、被多个脚本 source 的公共函数、
  契约对调用者有约束（参数顺序、退出码语义、stdout 格式）的函数。
* **日常档（一行摘要）**：脚本内部辅助函数、只在本文件用一次的小函数、
  逻辑一眼能看懂的封装。

判断标准：**别人会不会跨文件调它？** 会 → 完整档；不会 → 日常档。

### 2.1 完整档：shdoc 风格

标签紧贴函数上方，顺序为 `@description` → `@arg` → `@stdout` → `@exitcode`。
按需补 `@set`（设置的全局变量）、`@stderr`、`@example`。

```bash
# @description 解析原始用户 ID，校验为纯数字。
# @arg $1 string 原始用户 ID 字符串
# @stdout 解析成功后的整数用户 ID
# @exitcode 0 解析成功
# @exitcode 1 输入为空或含非数字字符
parse_user_id() {
    local raw="$1"
    [[ "$raw" =~ ^[0-9]+$ ]] || return 1
    printf '%s\n' "$raw"
}
```

要点：

* `@arg` 写 `$1 类型 语义`，可变参数用 `$@`：`# @arg $@ string 待链接的文件列表`。
* `@stdout` 写**格式契约**（是 JSON?一行?多行?），调用者据此解析。
* `@exitcode` 把每个有意义的退出码都列出，尤其失败码——这是 shell 的「Raises」。
* 函数有副作用（改全局变量、写文件）时用 `@set` / 在 `@description` 里点明。

带 `@set` 与多输出的示例：

```bash
# @description 探测默认出口网卡，结果写入全局 IFACE。
# @set IFACE string 默认路由对应的网卡名，无默认路由时为空
# @stdout 无
# @exitcode 0 找到网卡
# @exitcode 1 无默认路由
detect_iface() {
    IFACE=$(ip route 2>/dev/null | awk '/^default/ {print $5; exit}')
    [[ -n "$IFACE" ]]
}
```

### 2.2 日常档：一行摘要 + 必要参数说明

内部辅助函数只需一行说明用途，参数不直观时补一句即可。

```bash
# cd 后自动 ls（原 cd-ls 插件）
_dots_cdls() { [[ -o interactive ]] && eval "${CD_LS_COMMAND:-ls}" }

# 复制路径到剪贴板；$1 缺省为当前目录
copypath() {
    local target="${1:-.}"
    realpath -- "$target" | tr -d '\n' | wl-copy
}
```

日常档**不要**硬套 shdoc 标签——给三行小函数写五行 `@arg`/`@exitcode` 是噪音。

---

## 3. 全局变量与环境变量声明

在声明处注明语义、默认值、以及 `readonly` / `export` 的意图。

```bash
# 仓库根，由 bootstrap 注入；下游所有路径以此为基准
export DOTFILES_DIR="${DOTFILES_DIR:-$HOME/dotfiles}"

# 链接台账路径，常量不可变，避免被子函数误改
readonly STATE_FILE="$DOTFILES_DIR/.dots/state.json"

# 备份目录带时间戳，覆盖普通文件前先归档到这里
backup_dir="$DOTFILES_DIR/backup/$(date +%Y%m%d-%H%M%S)"
```

要点：

* `export` 要说明「为什么需要被子进程看到」（如供 systemd unit、子脚本读取）。
* `readonly` 要说明「为什么不可变」（常量、防误改）。
* 带 `${VAR:-default}` 的默认值，注释写清缺省行为，而不是重复那个值。
* 一组相关变量可合并一段注释，不必逐行。

---

## 4. 复杂管道 / awk / sed 一行流

对管道、`awk`/`sed`/`jq` 一行流，**上方写「做什么 / 为什么」**，不要逐字翻译命令。
读者能看懂语法，看不懂的是意图。

```bash
# 取默认路由的出口网卡名（default 行第 5 列），无默认路由则为空
iface=$(ip route 2>/dev/null | awk '/^default/ {print $5; exit}')

# 从 nmcli 输出里捞当前已连接的 wifi 那一行（active=yes）
line=$(LANG=C nmcli -t -f active,ssid,signal dev wifi 2>/dev/null | grep '^yes' | head -1)

# 聚合脚本拍平：把 scripts/ 下所有 *.sh 软链进 .gen/scripts/，重名后者覆盖
fd -e sh . "$SCRIPTS_SRC" -x ln -sf {} "$GEN_DIR/{/}"
```

反面教材（逐字翻译，等于没写）：

```bash
# 用 awk 打印第 5 个字段       ← ✗ 语法谁都看得懂，没说为什么是第 5 列
iface=$(ip route | awk '{print $5}')
```

`LANG=C`、`2>/dev/null` 这类「附加行为」如果不显然，也顺手点一句：

```bash
# LANG=C 强制英文输出，避免 locale 导致 nmcli 字段名被翻译后匹配失败
line=$(LANG=C nmcli -t -f active,ssid dev wifi | grep '^yes')
```

---

## 5. 影响全局行为的语句

这类语句改变整个脚本的运行语义，**必须注明理由**，因为出问题时第一个要查的就是它们。

### `set` 选项

```bash
# 任一命令失败即退出 / 引用未定义变量报错 / 管道中任一环失败都算失败
# 安装脚本不容忍静默错误：宁可早退也不要带着半截状态继续链接
set -euo pipefail
```

需要临时关掉时也要说明：

```bash
# 这段允许命令失败（探测性调用），临时关掉 errexit
set +e
nmcli dev wifi rescan 2>/dev/null
set -e
```

### `trap`

`trap` 注册的清理/收尾逻辑离触发点很远，注释要说清「捕获什么信号、做什么」。

```bash
# 无论正常退出还是中断，都删除解密出的明文，避免 secrets 落盘残留
trap 'rm -f "$plaintext"' EXIT INT TERM
```

### `IFS` / `shopt` / `setopt`

```bash
# 按换行分割，避免文件名里的空格被拆成多个词
IFS=$'\n'

# 允许 glob 不匹配时返回空而非原样字符串，防止误把 '*.sh' 当文件名链接
shopt -s nullglob
```

---

## 6. Zsh 特有

### 6.1 alias

alias 注释写「展开成什么 / 解决什么」，尤其是不直观的目录跳转、循环生成的 alias。

```bash
alias dot='cd "$DOTFILES_DIR"'            # 跳到仓库根
alias -- -='cd -'                         # 单独的 - 回上一个目录

# 1-9 跳目录栈第 N 层（配合 auto_pushd）；循环批量生成，用完清理临时变量
for _idx in {1..9}; do alias "$_idx"="cd +$_idx"; done
unset _idx
```

### 6.2 hook 函数（precmd / chpwd / preexec）

hook 由 zsh 在特定时机自动调用，注释必须点明**挂在哪个 hook、触发时机**，否则
读者看不出这个函数何时跑。

```bash
# chpwd hook：每次切换目录后自动 ls（交互式 shell 才生效）
_dots_cdls() { [[ -o interactive ]] && eval "${CD_LS_COMMAND:-ls}" }
add-zsh-hook chpwd _dots_cdls

# precmd hook：每次出提示符前刷新窗口标题为当前目录
_dots_set_title() { print -Pn "\e]0;%~\a" }
add-zsh-hook precmd _dots_set_title
```

### 6.3 ZLE widget 与 keybinding

自定义 widget（`zle -N`）+ `bindkey` 是成对的，注释说清「按键 → 行为」。
连续的 `bindkey` 可在行尾标键名，免去读者背 escape 序列。

```bash
# 慢补全时先显示红点提示，再触发补全（原 COMPLETION_WAITING_DOTS）
_dots_complete_waiting() {
    print -Pn "%F{red}…%f"
    zle expand-or-complete
    zle redisplay
}
zle -N _dots_complete_waiting
bindkey '^I' _dots_complete_waiting

bindkey '^[[A' up-line-or-beginning-search   # ↑ 前缀历史搜索
bindkey '^[[H' beginning-of-line             # Home
bindkey '^[[3~' delete-char                  # Delete
```

### 6.4 补全函数（`_command` / compdef）

自写补全函数只需简要覆盖「补全谁、补什么」，无需逐个 `_arguments` 注释。

```bash
# dots 子命令补全：补一级子命令 + sync/adopt 的路径参数
_dots() {
    local -a subcmds=(sync status adopt doctor bootstrap)
    _arguments '1:command:(${subcmds})' '*:path:_files'
}
compdef _dots dots
```

### 6.5 `zstyle` 配置

`zstyle` 行的语义不在 key 名里，行尾或上方补一句即可。

```bash
zstyle ':completion:*' matcher-list 'm:{a-zA-Z}={A-Za-z}'   # 补全大小写不敏感
zstyle ':completion:*' menu select                          # 菜单式选择补全项
zstyle ':completion:*' list-colors "${(s.:.)LS_COLORS}"     # 补全列表按 LS_COLORS 着色
```

---

## 7. 不要写的注释（反例）

给自明语句写注释只会制造噪音。以下都**不要**写：

```bash
cd "$dir"            # 进入 $dir              ← ✗ cd 字面意思就是进入
i=$((i + 1))         # i 加 1                 ← ✗
echo "done"          # 打印 done             ← ✗
rm -f "$tmp"         # 删除临时文件 $tmp      ← ✗ 文件名已说明，rm 已说明动作
local name="$1"      # 把第一个参数赋给 name  ← ✗
```

什么时候这些反而**该**写？当存在「非显而易见的理由」时——理由才是注释的价值：

```bash
rm -f "$tmp"         # 即便不存在也不报错，errexit 下用 -f 防中断 ← ✓ 解释的是 -f 的意图
cd "$repo" || exit 1 # cd 失败必须中止，否则后续会污染调用者目录   ← ✓ 解释后果
```

---

## 8. 一致性约定

同一仓库内保持统一，避免风格漂移：

* 函数库统一用 shdoc 标签；脚本内部统一用一行档，不在同一文件混档。
* 文件头一律含「职责 + Usage + 依赖 + 环境变量」四要素（无则写「无」）。
* `set -euo pipefail` 在安装/聚合类脚本中默认开启并注明理由。
* 私有 zsh 函数统一加 `_dots_` 前缀（如 `_dots_cdls`），注释点明对应的原插件/来源。
* 注释只描述对调用者/维护者有用的语义、约束、副作用，不描述「如何实现」。
