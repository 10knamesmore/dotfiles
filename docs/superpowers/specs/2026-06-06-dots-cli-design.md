# dots —— dotfiles 管理 CLI 设计文档

日期：2026-06-06
状态：已与用户逐节确认定稿

## 0. 背景与目标

现状 `install.py`（717 行 Python）的核心痛点：

1. **每加一个非 XDG 规范的链接目标（如 `~/.claude/hooks/xxx`）都要改安装器代码**。各类 coding agent 配置目录均不守 XDG，这类需求高频出现。
2. 模板渲染（`*_TEMPLATE` 占位符 + str.replace）人体工学差：改配置要"改模板→跑渲染"两步，模板变量表硬编码在 Python 里。
3. 无 per-host 支持：显示器坐标、背光设备名、校园网账号等单机参数硬编码。
4. oh-my-zsh + `static/omz_custom` 维护负担：更新唠叨、vendor 结构丑。

新机制目标（用户拍板）：

- 实现载体：**Rust 自写 CLI（名 `dots`）**，源码作为 workspace 放仓内，新机用极薄 bootstrap.sh 引导 cargo 编译。
- 核心机制：软链接管理，仓库即真相源。
- 保留能力：路径注入（重新设计）、`~/.zshrc` stub 保护、scripts 聚合进 PATH、bootstrap 装机。
- 本期实现 per-host 支持与 secrets 不入库安置。
- 目录结构可大改、一步到位。
- 顺带完成 OMZ 退役（shell 栈自包含化）。

## 1. 心智模型（一句话）

> **镜像 = 声明：文件放进 `tree/` 的对应位置即链接，零配置；例外 = 手写 `dots.lua`（LuaLS 类型标注，编辑器补全）；机器 = 只碰 `.dots/state.json` 台账，永不编辑清单。**

三条线完全分离：人写的进 Lua，机器写的进 JSON，90% 的日常操作两者都不碰。

## 2. 仓库结构

```
dotfiles/
├── bootstrap.sh              # 极薄（~15 行）：装 rustup → cargo build → exec dots bootstrap
├── dots.lua                  # 例外清单（100% 人手编辑，LuaLS 类型补全，CLI 永不修改）
├── .luarc.json               # 指向 cli/lua-api/，让 LuaLS 对 dots.lua 生效
├── cli/                      # Rust workspace
│   ├── lua-api/dots.meta.lua #   ---@meta 类型标注（DSL 的"schema"，随仓库走）
│   └── crates/
│       ├── dots/             #   bin：命令行、彩色输出（mlua 沙箱求值清单）
│       └── dots-core/        #   lib：纯逻辑（清单求值结果→Plan 计算），无 IO 副作用，可单测
├── tree/                     # ★ 映射根：目录结构即链接声明，纯 $HOME 镜像
│   ├── home/                 #   → $HOME（跨平台）
│   ├── home.linux/           #   → $HOME（仅 Linux，条目级覆盖通用层）
│   └── home.macos/           #   → $HOME（仅 macOS）
├── scripts/                  # 脚本源
│   ├── common/  linux/  macos/
├── hosts/                    # per-host 资产
│   ├── files/<hostname>/     #   本机专属配置文件（如 monitors.conf）
│   └── secrets.age           #   age 加密 secrets（密文入库，明文永不入库）
├── packages/                 # bootstrap 清单（纯文本，不硬编码在代码里）
│   ├── pacman.txt  aur.txt  brew.txt  apt.txt  toolchains.toml
├── docs/
├── .gen/        (gitignore)  # 派生区：scripts/（聚合）、injected/（渲染产物）
├── .dots/       (gitignore)  # state.json 链接台账（谁建/何时/什么模式）→ undo/unlink/漂移检测
└── backup/      (gitignore)  # 时间戳备份（沿用现机制）
```

注：per-host 条件直接写在 dots.lua 的 `hosts{}` 块里（§10），不再有 `hosts/<hostname>.toml` 文件族与深合并语义。

注意：`static/` 目录随 OMZ 退役一并消失（见 §7）；`generated/` 被 `.gen/` 替代。

## 3. 映射规则（替代 install.py 全部分支逻辑）

**规则 1 —— 纯 $HOME 镜像**：
- `tree/home` → `$HOME`，所有平台。
- `tree/home.<os>` → `$HOME`，仅该平台；同一目标路径**平台层条目级覆盖通用层**（同现状行为），被遮蔽项 `dots status` 可见。
- 没有任何工具专属概念。`~/.claude/...` 就住在 `tree/home/.claude/...`。
- 真正非 `$HOME` 的目标（如 macOS `~/Library/Application Support/...`）用 dots.lua 的 `root(...)` 声明额外层，按需出现。

**规则 2 —— 链接粒度启发式**：
- 文件 → 直接链。
- **层根第一级目录是"容器"**：不链自身，下钻一层、逐子项链接。
- 第二级及更深的目录 → 整目录链。

| tree 路径 | 行为 | 说明 |
|---|---|---|
| `tree/home/.vimrc` | `~/.vimrc` → 链文件 | |
| `tree/home/.config/` | 容器，下钻 | `~/.config/nvim` 等各自成链（=现状） |
| `tree/home/.config/nvim/` | 整目录链 | 深度 2（=现状） |
| `tree/home/.claude/` | 容器，下钻 | skills/hooks/agents/settings.json 各自成链 |
| `tree/home/.claude/hooks/` | 整目录链 | `~/.claude` 本体保持真实目录，Claude Code 运行时数据（projects/、todos/）不进仓库 |

**规则 3 —— 清单覆盖**：启发式不对时在 dots.lua 写一条 `granularity(...)`（`mode = "dir"|"children"|"file"` + `ignore`）。

### 3.1 链接判定规则（sync 对每个期望目标位置 T 的处理 —— 也是无缝迁移的核心）

对计算出的每个期望链接，按目标位置 T 的当前状态决策：

| T 当前状态 | 处理 |
|---|---|
| 不存在 | 建链 |
| 符号链接，且 `readlink(T)` 落在 `$DOTFILES_DIR` 内（**含旧路径 general/ linux/ generated/，含已断链**） | **无条件 unlink 重建**——这是"我们管的链接"，指向新旧无所谓 |
| 符号链接，指向 `$DOTFILES_DIR` 外 | 报漂移，sync 询问（不静默动用户的别处链接） |
| 真实文件/目录 | 备份到 `backup/<ts>/` 后建链 |
| 期望是"容器目录"（children 模式），但 T 是整目录软链 | 先 unlink 软链 → mkdir 真实目录 → 逐子项建链（处理 skills 从整目录软链转 children 的结构性转换） |

关键点：**判定只看"是否落在仓库内"，不看指向新路径还是旧路径**。因此 `git mv` 重排目录后，旧链接（指向 `general/...`，此刻已断链）一律被识别为可重建，sync 一次性收敛到 `tree/...` 新目标——迁移因此无缝，无需手动清理旧链接。

### 3.2 文件管理决策树（一个目标文件该怎么管）

**这是给"人"看的决策指引，不是 CLI 的代码分支。** 铁律：dots 是"机制"（链接 / 钩子执行 / 写原语），不是"策略"（哪个工具怎么配）——**CLI 对 "ssh"/"git"/"claude" 这些名字一无所知，不含任何工具知识库**。否则加一个新工具就要改 Rust 代码，正是本项目要消灭的痛点。所有工具特定知识都活在两处、全由人掌握：仓库内容（config 里写不写 `Include`、settings.json 放哪些键）和 dots.lua（granularity / 钩子声明）。

因此下面四格对 CLI 而言只落到三个通用机制：①②=普通 ln（区别仅在你往 tree 放的文件内容，比如 config 里有没有手写一行 `Include`）；③④=dots.lua 里的 granularity / `on()` 钩子。第①格的 include 是工具自己启动时解析的，dots 全程只是链接了两个文件、不知道 `Include` 是个指令。

按"能否原生分层"和"是否被工具运行时回写"分流，从上到下命中即停：

1. **支持原生 include**（ssh_config 的 `Include`、gitconfig 的 `include.path`/`includeIf`）→ dots 只链片段文件，主文件入库时含一行 include，**绝不做文件内注入**。per-host 差异走 include 条件（`includeIf "gitdir:~/work/"`、`Include ~/.ssh/config.d/*`）或 hosts{} 选择性 link；机密走被 include 的 age 片段。范例：
   ```
   # ~/.ssh/config（入库）         # ~/.gitconfig（入库）
   Include ~/.ssh/config.d/*       [includeIf "gitdir:~/work/"]
                                       path = ~/.config/git/work.inc
   ```
2. **整文件、不被运行时回写**（vimrc/starship/kitty/hypr 等 95% 的 rc）→ 整文件 ln（§3 镜像）。读不到 shell env 的消费者（systemd unit 等）→ `.inject`(minijinja) 渲染（§6）。
3. **整文件、会运行时回写部分键**（Claude/VSCode settings.json）→ 两种手段二选一：
   - **(简) 双文件分层**：整文件 ln 入库（只放稳定键 + dots 想管的键如 hooks）+ 工具自己的 `*.local.json`(gitignore) 吸收 noisy/机器特定键，靠工具原生 merge。零 dots 解析。**前提**：赌工具迁移后不再回写主文件（Claude 实测迁移后概率大降但非零）。
   - **(稳) 读-改-写钩子**（§6.4）：dots 不整文件 ln，而是用生命周期钩子的幂等原语只认领自己那段（如 hooks），保留工具回写的其余键。**不赌工具行为**，代价是对该文件 dry-run 失真（§6.4 详述）。
4. **整文件、无 include、无 local 分层、会运行时回写** → 必须用 §6.4 读-改-写钩子。当前已知文件中较少落此格，但**不假设工具一定提供 local 兜底**，故此机制常备而非投机。

## 4. dots.lua（例外清单：Lua + LuaLS 类型标注，纯手编）

**决策**：清单用 Lua 而非 TOML，且 **CLI 永不编辑清单**（无 `dist add` 类命令，无 append-only 写入）。理由：

- 与用户技术栈一致（hyprland.lua、nvim 全家 Lua），零新语法。
- 条件逻辑原生 `if`/`hosts{}`，**整个 per-host 叠加合并语义归零**（原 TOML 设计需要 hosts/*.toml 文件族 + 深合并规则）。
- "机器写回 Lua 无 toml_edit 等价物"这一硬伤，通过**彻底去掉机器编辑**消灭而非绕开。
- 编辑体验优于 TOML：仓库自带 `cli/lua-api/dots.meta.lua`（`---@meta` 标注）+ `.luarc.json`，nvim/LuaLS 对 `dots.lua` 提供字段补全、签名提示、类型检查，写错字段当场标红。

**求值环境**：mlua（vendored，cargo build 自包含，需 C 编译器——bootstrap 装的 base-devel/Xcode CLT 已含）。沙箱：锁掉 io/os.execute 等副作用 stdlib，注入只读全局 `dots.host`、`dots.os`、`dots.home`。保证 dry-run 与 sync 求值确定一致。语法/类型错误编辑期由 LuaLS 拦，语义错误（src 不存在、目标冲突）运行期由 sync/doctor 报。

```lua
-- dots.lua —— 例外清单，预期长期 < 60 行

-- 链接粒度覆盖（启发式不对时才写）
granularity("home/.config/opencode", {
  mode = "file",               -- 逐文件链
  ignore = { "node_modules", "package.json", "bun.lock", ".gitignore" },
  -- opencode 在配置目录生成运行时垃圾，整目录链会拖进仓库视图
})
granularity("home.linux/.config/systemd/user", {
  mode = "file",               -- 只链 *.service/*.timer；.wants/ 由 systemctl 管（§10）
})
granularity("home/.claude", {
  mode = "children",           -- ~/.claude 本体保持真实目录，逐项链
  ignore = { "projects", "todos", "shell-snapshots", "statsig",
             "history.jsonl", "settings.local.json" },
  -- 排除 CC 运行时产物 + *.local.json（声明它归机器、不归仓库；不是声明 dots 去编辑它）
})

-- 生命周期钩子（§6.4）：加 hook = 把仓库定义的 hooks 段合并进 settings.json
on("post_link", function()
  dots.json.merge("~/.claude/settings.json", {
    hooks = { Stop = {{ hooks = {{ type = "command",
      command = "$HOME/.claude/hooks/on-stop.sh" }} }} },
  })  -- 读-改-写：只认领 hooks 键，保留 CC 回写的 model/effortLevel 等
end)

-- 一次性副作用动作（取代 install.py 硬编码）
on("post_sync", function()
  dots.run_once("zoxide-import", "zoxide import --from z $HOME/.z")
end)

-- 多目标分发：一份源多落点。接入新工具 = 在 to 里加一行（LuaLS 补全）+ dots sync
distribute("skills", {
  src  = "tree/home/.claude/skills",   -- 唯一真相源（主落点走镜像 → ~/.claude/skills）
  to   = { "~/.codex/skills", "~/.copilot/skills", "~/.config/opencode/skill" },
  mode = "children",           -- 逐 skill 链，防 codex 的 skills/.system 运行垃圾污染
})
distribute("agents", {
  src = "tree/home/.claude/agents",
  to  = {},
})

-- systemd user 单元：sync 时 systemctl --user enable（幂等）；.wants/ 软链不再入库
systemd_user { "napcat.service", "mihomo.service", "bsu-login.service", "bsu-login.timer" }

scripts { keep_tree = { "hypr" } }   -- hypr/ 保持树形（键位用 $scripts_dir/hypr/xxx.sh）

-- 非 $HOME 镜像目标才声明额外层（罕见）：
-- root("vscode", { path = "~/Library/Application Support/Code/User", os = "macos" })

-- per-host 见 §10 的 hosts{} 块
```

## 5. CLI 命令面

```
dots sync [--dry-run]      幂等收敛：建链/渲染注入/聚合脚本/systemctl enable/护 stub
dots adopt <path>...       ★ 收编：搬进 tree 正确位置 + 原地反链 + 记台账（永不写清单）
dots status                三态巡检：✔ 已链 / ~ 漂移 / ✘ 缺失 + 孤儿链接；非零退出码可挂 CI
dots doctor                深检：hosts{} 未覆盖当前机、注入变量未解析、distribute 目标父目录
                           缺失、脚本重名、被遮蔽条目、台账与磁盘漂移、清单建议未粘贴的漂移、
                           钩子认领的 keypath 是否仍在目标文件中（读-改-写漂移）。
                           另含只读提示：Claude permissions 运行时实际落点是 ~/.claude.json 的
                           projects[].allowedTools，入库 settings.json 的 permissions 只读不回填
dots undo                  撤销上一次 adopt/unlink 的文件搬运与链接（靠 .dots/state.json）
dots unlink <path> [--keep-in-repo]   解除纳管，文件搬回 $HOME
dots secret set|edit <key> age 加密读写 hosts/secrets.age
dots bootstrap             装机：packages/* + toolchains + 末尾自动 sync
dots completions zsh       补全
```

**CLI 永不编辑 dots.lua**。需要清单配合的场景（granularity、distribute 加落点、root 层），CLI **打印建议的 Lua 行**让用户粘贴（有 LuaLS 补全，粘贴即校验）；忘记粘贴造成的"已建链但未声明"漂移由 doctor 检出。`dist add` 命令因此取消——接入新工具 = dots.lua 的 `to` 列表加一行 + `dots sync`。

`adopt` 的智能（全部可被 `--layer/--mode` 参数覆盖）：
- 按路径自动推断层：`~/.claude/hooks/x` → `tree/home/.claude/hooks/`；平台歧义时询问。
- grep 到文件内含仓库绝对路径 → 提醒转 `.inject`（防 systemd unit 类文件换机失效）。
- 检测到目录含 node_modules/lock 文件 → 打印建议的 `granularity(...)` 行。
- 输出附 `dots undo` 提示（undo 只逆向文件搬运与链接，不涉清单——清单本来就没被动过）。

## 6. 路径注入：渲染降级为最后手段

**原则：配置文件只引用"安装后路径"（$HOME 侧）或自身相对路径，永不引用仓库内部路径。`$DOTFILES_DIR` 只用于真正指向仓库本身的东西（`dot` 别名、`.gen/scripts`）。**

**A. 环境变量吃掉 90%**：`dots sync` 写固定路径小文件 `~/.config/dots/env.zsh`：

```zsh
export DOTFILES_DIR="/home/wanger/dotfiles"
export DOTS_SCRIPTS="$DOTFILES_DIR/.gen/scripts"
path=($DOTS_SCRIPTS $path)
```

`.zshrc_dotfiles` 变回普通文件直接软链（不再 .template），首行 source 该 env 文件。现有 4 个模板变量全部消灭，编辑配置即生效。

**B. 读不到 shell 环境的消费者才渲染**：典型是 systemd unit 的 `ExecStart=`。约定：源文件带 `.inject` 后缀（如 `bsu-login.service.inject`），sync 时渲染，产物落 `.gen/injected/` 再链到目标。预计全仓只剩 1-2 个这类文件。

**渲染引擎用 minijinja**（纯 Rust、无 C 依赖、可自定义定界符），不自写 str.replace。模板语法 `{{ }}`，上下文：

```jinja
ExecStart=/usr/bin/python {{ DOTFILES }}/scripts/bsu_login.py \
  --username {{ secret.bsu_user }} --password {{ secret.bsu_pass }}
```

上下文键：`DOTFILES`/`SCRIPTS`（内置绝对路径）、`host.*`（当前机 vars）、`secret.*`（age 解密）。选 minijinja 而非自写替换的收益：缺变量自动报错（= doctor "未解析变量"检查免费拿到）、支持 `{{ x | default(...) }}` 与 `{% if %}`（per-host 片段可内联）。定界符冲突规避：`$VAR`/`${VAR}`（systemd）、`$var`（hypr）均与 `{{ }}` 不冲突；`screen-effects.glsl` 含大量 `{}` 但它走脚本运行期 sed、不经 inject，无碰撞——万一将来有 `{}` 密集文件需渲染，minijinja 可换定界符。

**C. Hyprland 兜底**：`hyprland.lua` 改 `os.getenv("DOTFILES_DIR")`，取不到时读固定兜底文件 `~/.config/dots/root`（sync 写入，一行仓库路径）——裸 TTY 启动 Hyprland 拿不到 zsh 环境也不断。

### 6.4 生命周期钩子与写原语（片段管理 + 副作用动作的统一逃生门）

**动机**：纯链接 + .inject 覆盖不了两类需求——(a) 往工具自己拥有、会运行时回写的文件里"只管一段"（如 Claude settings.json 的 hooks 键）；(b) sync 流程里的一次性副作用动作（`systemctl --user enable`、`zoxide import`、compinit 缓存重建）。这两类历史上都硬编码在 install.py 里，每加一个就改 Rust 代码——正是用户的核心痛点。dots 用"生命周期钩子 + 受限写原语"统一解决，且**不内建任何格式专属的声明式 merge DSL**（合并逻辑由用户在 Lua 里用原语自己写，dots 只提供安全的执行基建）。设计原则：不依赖工具是否提供原生分层（不赌工具行为），但把确定性损失约束到最小、显式标注。

**两阶段执行**（保住 §4 沙箱与 dry-run）：
- **声明阶段**：求值 dots.lua、构建链接 Plan。纯沙箱、零 IO、确定性，可离线单测（同 §13）。钩子在此阶段只被**注册**（记录闭包），不执行。
- **effect 阶段**：链接落盘后，按生命周期点依次执行已注册钩子。钩子在受限环境跑：**mlua 禁掉裸 `io`/`os`**，副作用只能经 `dots.*` 原语——因此所有写盘都受控、可 dry-run 拦截、可登记。

**生命周期点**：`pre_sync` / `post_link`（链接全部建好后）/ `post_sync` / `on_host_activate`。`on("post_link", fn)` 注册。

**写原语**（统一走 §12 plan/execute + 以下保证）：
- 共同保证：**atomic**（写临时文件 + rename）、写前**备份**到 `backup/<ts>/`、**无差异不写盘**（序列化后逐字节比对，相同则不触碰 mtime）、执行时**自登记 ownership** 到 `.dots/state.json`（哪个文件的哪个 keypath 归 dots 管）→ `doctor` 据此反向检测漂移。
- `dots.json.merge(path, table)` / `dots.json.set(path, keypath, value)`：解析目标 JSON → 在声明的 path 上深合并（保序、只动自己的 key、保留其余）→ 稳定键序序列化。`dots.toml.*` 同理。
- `dots.file.ensure_block(path, marker, content)`：文本文件的 managed-block（带注释标记的区间替换，幂等）。仅用于支持注释的格式（conf/ini），**不用于 JSON**（JSON 无注释，必须走 json.merge）。
- `dots.run_once(key, cmd)`：幂等执行一次性副作用命令，`key` 记入 state.json，已跑过则跳过（zoxide import / systemctl enable 这类）。

**两类钩子的确定性区别**（必须显式区分）：
- **生成型**（原语入参不读目标当前值，内容只来自仓库/host/secret）：确定性安全，dry-run 精确预测 diff。绝大多数副作用动作与"写 dots 完全拥有的片段文件"属此类。
- **读-改-写型**（`json.merge`/`json.set` 这类要读目标当前值以保留工具回写的其他键）：对该目标文件 **dry-run 失真**——因为目标正被工具并发写，dry-run 此刻读到的值与 apply 时不一定相同。dry-run 对这类只诚实声明意图（"will merge `hooks` into `~/.claude/settings.json`, preserve other keys"），不假装精确。这是"不赌工具行为"换来的、被显式标注的代价。安全/幂等仍由原语保证（atomic + 备份 + 无差异不写）。

**与 minijinja .inject 的边界**：`.inject` 用于 dots 完全拥有、整文件渲染的产物（systemd unit）；`dots.json/file` 原语用于"工具拥有、dots 只管一段"的文件。前者生成型确定性，后者可能读-改-写。

## 7. shell 栈：OMZ 退役与模块化

### 7.1 退役决策

- **oh-my-zsh 本体退役**（提示符早已是 starship，主题层从未使用）。
- **you-should-use 删除**（用户判定无用）。
- **correction（setopt correct_all）删除**。
- **zsh-syntax-highlighting：整库冻结 vendor**（要完整功能、不再跟上游更新；它本是独立插件不依赖 OMZ）。
- **z → zoxide**（进 packages/ 清单；一次性 `zoxide import --from z ~/.z` 迁移跳转历史）。
- copypath / copyfile / cd-ls / zsh-allclear / waiting-dots → 内联微函数（waiting-dots 保留，约 3 行）。
- `static/omz_custom` 删除，`static/` 目录消失；install.py 的 bootstrap_omz 步骤随之消亡。

### 7.2 模块化结构（conf.d 数字前缀）

```zsh
# .zshrc_dotfiles 里的加载逻辑
for _f in "$HOME"/.config/zsh/*.zsh(nN); do source "$_f"; done
```

```
tree/home/.config/zsh/
├── 10-options.zsh                  # OMZ 后果补齐表全在这（见 7.3）
├── 20-functions.zsh                # cd-ls/allclear/copypath/copyfile/waiting-dots
├── 90-syntax-highlighting.zsh      # 2 行自相对 shim：source "${0:A:h}/vendor/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh"
└── vendor/zsh-syntax-highlighting/ # 冻结整库（目录不匹配 *.zsh，不被循环加载）
```

`(nN)` = 数字排序 + nullglob；编号保证顺序，z-sy-h 的 90 号天然最后加载（其硬性要求）。新增 shell 功能 = 扔一个编号 .zsh 文件，不碰入口。

### 7.3 OMZ 隐性依赖补齐表（10-options.zsh 的验收清单）

| # | 丢失项 | 补救 |
|---|---|---|
| 1 | `HISTFILE`（OMZ 设的；不补则历史不落盘） | `HISTFILE=~/.zsh_history` |
| 2 | share_history / hist_ignore_dups / extended_history | ~4 行 setopt |
| 3 | ↑↓ 前缀历史搜索、Home/End/Delete 键 | ~10 行 bindkey（up-line-or-beginning-search 等） |
| 4 | compinit + 大小写不敏感匹配 + 菜单高亮 + 补全缓存 | ~8 行；`uv generate-shell-completion` 依赖 compinit 先跑，顺序排好 |
| 5 | `..` `...` `-` `1`-`9` 目录别名、auto_cd、auto_pushd | ~6 行 |
| 6 | interactive_comments（粘贴带 # 的命令） | 1 行 setopt |
| 7 | 终端标题自动更新 | kitty shell integration 已兜底，不补 |
| 8 | z 跳转数据库 | `zoxide import --from z ~/.z` 一次性迁移 |
| 9 | take 等零散函数 | 用到啥补啥 |

迁移验收按此表逐项手测（双终端验历史共享、前缀↑、Tab 大小写匹配等），不以"能跑"为准。

## 8. ~/.zshrc stub（沿用并加固）

```
# DOTS_MANAGED: 由 dots 维护。下面这行勿删；其余内容（conda/nvm 等追加）安全保留。
source "$HOME/.zshrc_dotfiles"
```

- 首行 marker 命中（新 `# DOTS_MANAGED:` **或旧 `# DOTFILES_MANAGED:`**）→ 只确保 source 行存在（缺则幂等补回），绝不动其他行。机器本地工具初始化放 `~/.zshrc` 不入库的习惯被保护。**兼容旧 marker 是无缝迁移的必要条件**：否则 dots 会把 install.py 时代留下的 `~/.zshrc` 当陌生文件备份接管。命中旧 marker 时顺手升级为新 marker。
- 普通文件无 marker → 备份后接管；软链 → 重建 stub（同现状）。
- 现有 `~/.zshrc_dotfiles → generated/.zshrc_dotfiles`（渲染产物）：迁移后目标变为 `tree/home/.zshrc_dotfiles`（普通文件，§6 已消灭渲染），由 §3.1 判定（旧软链落在仓库内）无条件重建到新目标。

## 9. scripts 聚合

- `scripts/common/ + scripts/<os>/` → 聚合软链到 `.gen/scripts/`：顶层文件拍平；`[scripts] keep-tree` 列表里的子目录（hypr/）保持树形，不破坏键位 `$scripts_dir/hypr/xxx.sh` 引用。
- 重名冲突 → doctor 报错（修正现状的静默覆盖）。
- 新脚本丢进目录，sync 即进 PATH。

## 10. per-host 与 secrets

**per-host 写在 dots.lua 的 `hosts{}` 块**（不再有 hosts/*.toml 文件族与合并语义）：

```lua
hosts {
  xz07 = function()
    vars { backlight = "amdgpu_bl1", ddc_index = "1" }   -- 供 .inject 文件引用
    link("hosts/files/xz07/monitors.conf", "~/.config/hypr/monitors.conf")  -- 纯链接
  end,
  -- 当前 hostname 未在表中 → sync 硬报错，不静默回落（堵死"渲错显示器坐标不报错"）
  -- 显式逃生门：default = function() end
}
```

新机器首次 sync 报错时，CLI 打印建议的 `hosts{}` 骨架（含待填的 vars 键名）供粘贴。

显示器等整段配置**优先走"per-host 文件 + 主配置 source 引用"纯链接方案**（hyprland.lua 加 source monitors.conf），不走变量渲染——hostname 失配最坏是文件缺失报错，绝不静默渲出错误值。

**secrets**：`hosts/secrets.age`（age 加密，公钥入库私钥不入库），`dots secret set <key>` 写入，`.inject` 文件里 `{{ secret.key }}` 引用。诚实声明：age 只保护 git 同步面；渲染产物（如 bsu-login.service）在本机 `.gen/injected/` 仍是明文（systemd 必须读明文）。

迁移现状（2026-06-06 已完成第一步）：校园网明文密码已移出仓库——`bsu-login.service` 改用 `EnvironmentFile=%h/.config/bsu/credentials.env`（0600，不入库），git 全历史已 `git-filter-repo` 清洗。dots secrets 机制落地后，这个 EnvironmentFile 方案可平滑替换为 `{{ secret.bsu_pass }}` 注入（二选一即可，EnvironmentFile 本身也是合法长期方案）。

**systemd enable 状态**：`.wants/` 软链是 enable 状态载体（绝对路径、机器相关），**不再当文件纳管入库**。改为 `[systemd] user-units` 声明，sync 时 `systemctl --user enable`（幂等）。现暂存的 napcat `.wants` 链接迁移时删除。

## 11. bootstrap（新机一条命令）

```bash
git clone <repo> ~/dotfiles && ~/dotfiles/bootstrap.sh
```

bootstrap.sh（~15 行 sh）：有 cargo 跳过 rustup → `cargo build --release` → `exec dots bootstrap`。
`dots bootstrap`（Rust 内，可测）：探测 backend（pacman/brew/apt）→ 装 `packages/*.txt`（加包改文本不改代码）→ 装 toolchains（uv/starship/zoxide/nvm…，逐项探测幂等跳过）→ 自动 `dots sync` → 提示建 host 文件。omz 安装步骤删除。

## 12. 备份与错误处理

- 覆盖普通文件/目录前 → `backup/<ISO时间戳>/`（保留 .config 二级结构，同现状）；目标是软链 → 直接重建不备份（同现状）。
- **plan/execute 两段式**：dots-core 先算完整 Plan（建链/删除/备份/渲染清单），executor 再落盘。`--dry-run` = 只打印 Plan。Plan 可直接单测断言。
- 致命错误（建链失败）非零退出；可选目标缺失（AI 工具未装 → distribute 父目录不存在）warn 跳过（同现状容错哲学）。

## 13. 测试策略

- `dots-core`：纯逻辑单测（清单求值、粒度启发式、层覆盖、Plan 计算），proptest 跑路径边界；mlua 沙箱确定性测试（同输入两次求值结果全等、副作用 stdlib 不可达）。
- `dots` bin：assert_cmd + 临时 $HOME 跑 e2e（adopt→status→undo 全链路），insta 快照锁 status 输出格式。
- 钩子与写原语：生成型钩子可纯单测（输入→输出确定）；读-改-写原语用 fixture 目标文件测幂等（`fn(fn(x))==fn(x)`）、保序、无差异不写盘、atomic 失败回滚；沙箱测裸 `io`/`os` 不可达。
- **迁移对拍闸**：`dots sync --dry-run` 链接集合与 `install.py --dry-run` 全等（target→source 逐对比对），通过才允许切换。

## 14. 迁移路径（与现有 install.py 链接无缝衔接）

### 14.0 现状基线（2026-06-06 实测）

现有链接全部是**绝对路径**指向仓库内 `general/`、`linux/`、`generated/`：

```
~/.config/nvim    → general/.config/nvim     ~/.config/hypr   → linux/.config/hypr
~/.zshrc_dotfiles → generated/.zshrc_dotfiles ~/.alias        → general/.alias
~/.claude/skills  → general/skills（整目录软链） ~/.claude/agents → general/agents/claude
~/.codex/skills · ~/.copilot/skills → general/skills（整目录软链）
~/.zshrc          首行 marker = `# DOTFILES_MANAGED:`（install.py 时代）
```

新机制目标路径全变（`tree/...`）。无缝衔接靠三个机制保证，**无需手动清理任何旧链接**：
- §3.1 链接判定：旧链接落在仓库内 → 无条件重建到新目标（含 git mv 后的断链态）。
- §8 marker 兼容：dots 识别旧 `# DOTFILES_MANAGED:`，不会把 `~/.zshrc` 误当陌生文件备份。
- skills 结构转换：§3.1 末行处理"整目录软链 → children 真实目录 + 逐项链"。

### 14.1 分阶段（每步可回退，install.py 暂存共存）

1. `cli/` 落地，先实现 sync/status 核心引擎 + §3.1 判定（TDD）。
2. **对拍闸先行**：在重排前，让 `dots sync --dry-run` 读"映射后等价结构"产出的链接集合与 `install.py --dry-run` 全等（target→source 逐对，允许 source 路径 general/→tree/ 的已知重写）。通过才动目录。
3. `git mv` 重排目录：`general/.config/*`→`tree/home/.config/*`、`linux/.config/*`→`tree/home.linux/.config/*`、`general/skills`→`tree/home/.claude/skills`、`general/agents/claude`→`tree/home/.claude/agents`、`*/scripts`→`scripts/<scope>/`…（保历史）。
4. **原子切换**：步骤 3 与 `dots sync` 在同一会话内连续完成，中间**不重启 WM / 不重连 session**（git mv 后到 sync 前所有链接处于断链窗口期，重启 Hyprland 会黑屏）。sync 后立即 `dots status` 全绿验证，再继续。
5. 模板消灭：`.zshrc_dotfiles.template` → 普通文件 `tree/home/.zshrc_dotfiles` + `$DOTFILES_DIR`；写 `~/.config/dots/env.zsh`；`hyprland.lua` 改 getenv+兜底文件 `~/.config/dots/root`。
6. shell 栈改造：OMZ 退役、conf.d 模块化、§7.3 补齐表逐项落地与手测、zoxide 数据迁移。
7. systemd：`.wants` 链接删除（含暂存的 napcat 那条），改 `[systemd]` 声明；bsu-login 已用 EnvironmentFile（§10），可后续转 `dots secret`。
8. per-host：显示器/背光/ddc 抽进 dots.lua 的 `hosts{}` 块 + `hosts/files/<host>/monitors.conf`。
9. 全绿后删 `install.py`、`generated/`、`static/`，更新 CLAUDE.md/README 对应章节。

### 14.2 回退

任意阶段失败：`git reset --hard` 回退目录重排（git mv 可逆），重跑旧 `install.py` 即恢复旧链接（旧链接判定同样把 tree/ 链接视为仓库内可重建，install.py 的 backup_file 对 symlink 直接重建）。两套安装器在切换期对同一组目标位置幂等收敛，互不毁坏数据。

## 15. 三场景演练

**A. 给 Claude Code 全局加 hook**（已核实：hook 须在 `~/.claude/settings.json` 的 `"hooks"` 键注册，CC 不扫描 hooks 目录；落第 3/4 格——工具会运行时回写 settings.json）：

- 脚本：写在 `tree/home/.claude/hooks/on-stop.sh`，进 tree 即被 §3 链接到 `~/.claude/hooks/`（`adopt` 也行）。
- 注册：在 dots.lua 写一个 `on("post_link", ...)` 用 `dots.json.merge` 把 hooks 段合并进 `~/.claude/settings.json`（见 §4 示例）。**读-改-写型**：只认领 hooks 键，CC 回写的 model/effortLevel 等原样保留——dots 不赌"CC 不碰 settings.json"，主动只管自己那段。
- `dots sync` → 钩子幂等合并、atomic 写、无差异不写盘 → commit。代价：该文件 dry-run 只声明意图不给精确 diff（§6.4）。
- 这条把"加 hook"压成"写脚本 + dots.lua 加一段 merge + sync"，0 安装器代码改动；且不依赖 CC 是否提供 local 分层。

**B. 新装 Linux 机器**：`git clone … && ./bootstrap.sh` → 装包+工具链自动完成 → 首次 sync 报"hostname 未覆盖"并打印 `hosts{}` 骨架 → 粘进 dots.lua 填显示器/背光 → `dots secret set bsu_pass` → `dots sync && exec zsh`。

**C. opencode 接入 skills**：dots.lua 里 `distribute("skills").to` 加一行（LuaLS 补全）→ `dots sync` → 完（对比现状要改 link_skills() 的 Python 元组与安装器代码）。

## 16. 已知弱点（诚实清单）

- 链接来源（镜像约定/distribute/inject）+ 钩子写原语并存，CLI 实现复杂度最高，doctor 必须同时理解——用强测试基建对冲。
- 改 CLI 要重编译，迭代环比 Python 长；mlua vendored 构建需 C 编译器（bootstrap 已覆盖）。
- adopt 推断可能猜错（有 --layer/--mode 覆盖 + undo 兜底）。
- 清单 100% 手编是刻意决策：CLI 打印建议行但不能替你粘贴，"忘记粘贴"造成的漂移要靠 doctor 检出（非阻断）。
- **读-改-写钩子的确定性让步**：对工具并发回写的文件（settings.json），dry-run 对该文件失真、只声明意图——这是"不赌工具原生分层"换来的、被显式标注的代价。安全/幂等由原语兜（atomic+备份+无差异不写），但 Plan 对这类 target 必须在线读盘，破坏 dots-core "纯离线可单测"的纯度（仅限这类 target）。
- 写原语库要为每种格式（JSON/TOML/文本块）各实现一套保序、最小-diff 的合并器，是实打实的 Rust 工作量。
- 钩子里仍可写出逻辑 bug（条件写错），破坏面被"禁裸 io + 原语 atomic + 备份"限制住，但非零。
- Lua 清单只有跑解释器才能读，第三方工具不能像 TOML 一样直接解析（对单人仓库影响极小）。
- age 私钥安全到达新机无银弹（手动拷贝/密码管理器）。
- z-sy-h 冻结后不获上游修复；兼容问题出现时退路是 packages/ 加官方包一行。

## 17. 范围外（明确不做）

- 包管理（除 bootstrap 装机清单外的日常包同步）——历史上做过又删除的决策维持。
- git 历史明文密码清洗：**已完成**（2026-06-06 filter-repo + force push，全历史无明文）。
- 多机 git 协作冲突的结构化 merge 工具。
- 本机静止数据加密（.gen/injected 明文是机制下限）。
- dots **不内建格式专属的声明式 merge DSL**：JSON/TOML 的合并由用户在钩子里用 `dots.json.*` 原语自己写。dots 只提供安全执行基建（两阶段、atomic、幂等、自登记），不替用户决定"哪些 key 怎么合"。
