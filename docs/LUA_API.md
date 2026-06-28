# dots.lua API 参考

`dots.lua` 是仓库根的**例外清单**：镜像约定盖不住的才写这里，预期长期 < 60 行。本文是它全部 Lua API 的参考文档——每个 API 都给出「输入长什么样 → 产物长什么样」的图示。

## 0. 心智模型

### 定位：例外，不是配置

`dots` 的主体机制不需要任何配置——`tree/` 的目录结构本身就是链接声明。先看默认规则把一棵树变成什么：

```text
仓库侧                                    $HOME 侧（dots sync 之后）
══════                                    ══════
tree/home/
├── .vimrc                                ~/.vimrc ───────────→ tree/home/.vimrc        ① 文件：直接链
├── .config/                              ~/.config/                                    ② 层根一级目录＝「容器」：
│   ├── starship.toml                     ├── starship.toml ──→ tree/home/.config/starship.toml
│   ├── nvim/                             ├── nvim ───────────→ tree/home/.config/nvim  ③ 二级及更深目录：整目录一根链
│   │   └── init.lua                      │                       （nvim 目录本身是 symlink）
│   └── kitty/                            └── kitty ──────────→ tree/home/.config/kitty
│       └── basic.conf
└── .agent/                               ~/.agent/                                     （.agent 也是层根一级目录
    └── skills/                           └── skills ─────────→ tree/home/.agent/skills    → 真实目录、下钻逐子项链）
```

三条启发式（`cli/crates/dots-core/src/layer.rs:127-131`）：

1. **文件**直接链（①）
2. **层根的一级目录**（`.config/`、`.agent/`）是「容器」：保持真实目录，下钻逐子项链（②）
3. **二级及更深的目录**整目录一根链（③）

`dots.lua` 只在三种情况出场：

1. 默认规则**猜错了**（粒度不合适 → `granularity`）
2. 默认规则**做不到**（一源多落点 → `distribute`；$HOME 外的目标 → `root`；机器差异 → `hosts`）
3. 链接之外的**副作用**（enable systemd 单元 → `systemd_user`；任意自定义 → `on` 钩子）

CLI **永不编辑这个文件**。需要清单变更时（比如 `dots adopt` 发现值得加一条粒度覆盖），它打印建议的 Lua 行让你自己粘贴。

### 两阶段执行

理解 API 的关键是 sync 的两阶段模型：

```text
┌─ 声明期（eval）──────────────────────────────┐
│ mlua 沙箱执行 dots.lua                        │
│ · granularity/distribute/... 只是「登记意图」  │
│ · on()/hosts{} 的闭包只存句柄，不执行          │
│ · 纯净、确定：同样输入永远同样 Manifest        │
└──────────────────────────────────────────────┘
                     ↓
┌─ effect 期（执行计划时）──────────────────────┐
│ · 命中的 hosts 块闭包此时才执行                │
│ · 全局钩子按 pre_sync → on_host_activate →     │
│   post_link → post_sync 触发；条目级 pre/post  │
│   夹着链接执行（完整时间线见 on() 一节）        │
│ · vars/link/dots.json/dots.file/dots.run(_once)│
│   等写原语此时才被注入，仅在闭包内可用          │
└──────────────────────────────────────────────┘
```

这个分离保证了 `dots sync --dry-run` 的确定性：声明期跑完就知道完整计划，不会因为执行了半个钩子而产生副作用。

> 实现：`cli/crates/dots/src/lua/eval.rs`（声明期）、`cli/crates/dots/src/lua/api.rs`（DSL 注册）、`cli/crates/dots/src/lua/primitives.rs`（effect 写原语）。

### 沙箱

声明期的 Lua 环境移除了 `io`、`os`、`require`、`dofile`、`loadfile`、`load`、`loadstring`、`package`（`eval.rs:60-75`）。`dots.lua` 里写 `os.getenv(...)` 会直接报错——需要环境信息用只读的 `dots.host` / `dots.os` / `dots.home` / `dots.repo`（见 §3）。

### 编辑器支持

`.luarc.json` 把 `cli/lua-api/dots.meta.lua`（LuaLS 类型标注）挂进 workspace，nvim 里编辑 `dots.lua` 有字段补全、签名提示、类型检查。改 CLI 的 API 时同步维护 meta 文件。

---

## 1. 声明类 API（顶层调用）

### `granularity(path, spec)` — 覆盖链接粒度

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `path` | string | **相对 `tree/` 的路径**（带层前缀），如 `"home/.config/opencode"`、`"home.linux/.config/systemd"` |
| `spec.mode` | `"dir"` \| `"children"` \| `"file"` | 缺省 `"dir"` |
| `spec.ignore` | string[] | 下钻时跳过的子项名（不链接、不管理） |
| `spec.pre` | fun(): boolean? | 条目级钩子：链接该条目**前**执行；`return false` → 整条目跳过（见下） |
| `spec.post` | fun() | 条目级钩子：该条目链接完成**后**执行；被 pre 阻止则不执行 |

三种 mode 用同一个输入对比。仓库里的真实案例——opencode 会往自己的配置目录写运行时垃圾：

```text
仓库侧（tree/home/.config/opencode/）
├── opencode.json          ← 想管理的配置
├── tools/
│   └── git-worktree.ts    ← 想管理的自定义 tool
└── （opencode 运行时还会生成 node_modules/、bun.lock …）
```

`.config/opencode` 在二级深度，默认规则是 ③ 整目录链（`mode = "dir"`）：

```text
mode = "dir"（缺省）——目录本身是一根 symlink
═══════════════════════════════════════════
~/.config/
└── opencode ─────────────→ tree/home/.config/opencode
    ├── opencode.json            （透过链接看到的内容）
    ├── tools/
    └── node_modules/  ✗ opencode 生成的垃圾穿过链接直接落进仓库！
```

改成 `children`——目录保持真实，下钻一层逐子项链：

```text
mode = "children"
═══════════════════════════════════════════
~/.config/
└── opencode/                          （真实目录）
    ├── opencode.json ────→ tree/home/.config/opencode/opencode.json
    ├── tools ────────────→ tree/home/.config/opencode/tools     （子目录仍是整链：
    └── node_modules/      （ignore：留在 $HOME 侧 ✓）              opencode 若往 tools/ 里
                                                                   写垃圾，仍会穿进仓库）
```

改成 `file`——递归到底，只有文件是链，所有中间目录都真实：

```text
mode = "file" + ignore
═══════════════════════════════════════════
~/.config/
└── opencode/                          （真实目录）
    ├── opencode.json ────→ tree/home/.config/opencode/opencode.json
    ├── tools/                         （真实目录）
    │   └── git-worktree.ts ─→ tree/home/.config/opencode/tools/git-worktree.ts
    └── node_modules/      ✓ ignore 列表里：dots 完全不碰，
                             垃圾留在 $HOME 侧，仓库永远干净
```

这正是 `dots.lua` 里的写法：

```lua
granularity("home/.config/opencode", {
  mode = "file",
  ignore = { "node_modules", "package.json", "bun.lock", ".gitignore" },
})
```

**怎么选 mode：**

| 情况 | mode |
| --- | --- |
| 目录完全归 dots 管，外部程序不写入 | `dir`（缺省，最省事：一根链全搞定） |
| 目录下既有受管项、外部程序又会**添加新条目**（如 systemctl 写 `*.wants/`、CC 写 `projects/`） | `children` |
| 连子目录内部都会被外部程序写垃圾 | `file` + `ignore` |

再看一个嵌套覆盖的真实案例——systemd：`systemctl --user enable` 会在 `user/` 里创建 `*.wants/` 目录，这些机器状态不该进仓库：

```lua
granularity("home.linux/.config/systemd", { mode = "children" })  -- systemd/ 下钻
granularity("home.linux/.config/systemd/user", {                  -- user/ 逐文件
  mode = "file",
  ignore = { "default.target.wants", "timers.target.wants" },
})
```

```text
~/.config/systemd/                     （真实目录，来自外层 children）
└── user/                              （真实目录，来自内层 file）
    ├── mihomo.service ───→ tree/home.linux/.config/systemd/user/mihomo.service
    ├── napcat.service ───→ tree/home.linux/.config/systemd/user/napcat.service
    ├── bsu-login.timer ──→ tree/home.linux/.config/systemd/user/bsu-login.timer
    └── timers.target.wants/   ✓ systemctl enable 写的真实目录，留在本机
        └── bsu-login.timer → ../bsu-login.timer
```

**条目级 pre/post 钩子**——钩子跟着声明走，比散在全局 `on{}` 里可读性高：

```lua
granularity("home/.config/opencode", {
  mode = "file",
  ignore = { "node_modules" },
  pre = function()
    return dots.os == "linux"   -- false → 整条目跳过；nil/true/其他 → 继续
  end,
  post = function()
    dots.run_once("opencode-init", "…")
  end,
})
```

```text
pre 返回值 → 条目命运
═══════════════════
return false        → ⊘ 该条目展开的全部链接从计划剔除，post 不执行
return true / nil   → 正常链接，post 在 execute 后执行
（无 return 即 nil）→ 同上——忘写 return 不会意外阻断
```

- pre 在「收集链接」阶段评估（时间线③），post 紧跟 execute（时间线⑤，在 `.inject` / 全局 post_link 之前）。
- 多个条目的 pre/post 相互独立，之间不保证执行顺序。
- **dry-run**：pre 照常评估（预览才准确；闭包内写原语自带 dry-run 感知），post 不执行（效果未发生）。
- `dots status` / `dots doctor` 是只读巡检、**不执行 pre**——被 pre 长期阻止的条目会在 status 里报缺失（已知限制）。

> ⚠️ **坑**：`mode` 拼错**不报错**，静默回落到 `"dir"`（`api.rs:35-41` 的 `parse_mode` 用 `_ =>` 兜底）。写 `mode = "chidlren"` 会得到整目录链。LuaLS 的类型检查能在编辑器里标出来——这是 meta 文件存在的另一个理由。

### `distribute(name, spec)` — 一源多落点

镜像规则是一对一的（`tree/home/X` → `$HOME/X`）。同一份内容要落到多个地方时用它。

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `name` | string | 分发组标识，仅用于人类可读输出——目前只有 pre 跳过提示用它（`sync.rs` 的「⊘ 分发跳过（pre）：\<name\>」）；不参与链接判定、无重名检测 |
| `spec.src` | string | 源，**相对仓库根**（注意带 `tree/` 前缀，和 `granularity` 不同） |
| `spec.to` | string[] | 落点列表（$HOME 侧，可用 `~`） |
| `spec.mode` | 同 granularity | 落点粒度，缺省 `"dir"` |
| `spec.pre` | fun(): boolean? | `return false` → 整个分发跳过（语义同 granularity 的 pre） |
| `spec.post` | fun() | 分发链接完成后执行；被 pre 阻止则不执行 |

真实案例——skills 一份源喂三个 AI 工具（skills 是公开标准、不专属某个工具，源住中立的 `~/.agent/skills`，Claude 和 codex/copilot 一样是订阅者）：

```lua
distribute("skills", {
  src = "tree/home/.agent/skills",
  to = { "~/.claude/skills", "~/.codex/skills", "~/.copilot/skills" },
  mode = "children",
})
```

```text
仓库侧（唯一真相源）                  $HOME 侧
══════════════════                  ══════
tree/home/.agent/skills/
├── rust-tdd/          ┌──────────  ~/.agent/skills ←──── 源的镜像家：整目录链（不归 distribute 管）
├── quickshell/        │
├── gh-cli/            │            ~/.claude/skills/           （mode=children：真实目录，逐 skill 链）
└── …                  ├──────────  ├── rust-tdd ────→ tree/home/.agent/skills/rust-tdd
                       │            ├── quickshell ──→ tree/home/.agent/skills/quickshell
                       │            └── my-local-skill/    ← 仓库外的本地 skill，sync 不碰
                       │
                       ├──────────  ~/.codex/skills/         （同上）
                       └──────────  ~/.copilot/skills/       （同上）
```

为什么落点用 `children` 而不是 `dir`：① 落点保持真实目录，本地（不入库的）skill 可与受管链接共存；② codex 会往 skills 目录写 `.system/` 之类的运行时产物——逐 skill 链让垃圾留在落点真实目录里，不污染源。

行为细节：

- **落点父目录不存在 → warn 并跳过**（`cmd/sync.rs` 的 `distribute_links`）。这台机器没装 codex（`~/.codex/` 不存在）时 sync 不会失败，装上后再 sync 即生效。
- 接入新工具 = `to` 加一行 + `dots sync`。

### `root(name, spec)` — 声明 $HOME 之外的映射层

默认只有 `tree/home*` → `$HOME` 这组映射。要镜像到别处时声明新层：

```lua
root("appsupport", {
  path = "~/Library/Application Support",
  os = "macos",          -- 仅该平台生效；省略则全平台
})
```

```text
仓库侧                                $HOME 之外的目标根
══════                                ══════════════
tree/appsupport/                      ~/Library/Application Support/
└── Code/                             └── Code/
    └── User/                             └── User/
        └── settings.json ──────────────→     └── settings.json → tree/appsupport/Code/User/settings.json
```

声明之后 `tree/appsupport/` 享受和 `tree/home/` 完全相同的待遇（启发式、granularity 覆盖、备份、漂移检测）。罕用——当前 `dots.lua` 没有用到，API 留作 macOS App Support 这类场景。

### `systemd_user(units)` — sync 时 enable 单元

unit 文件链到 `~/.config/systemd/user/` 只是文件就位；还要 `systemctl --user enable` 才挂上 target。写在这里的单元在 sync 末尾被 enable（幂等，反复 sync 不报错）。

```lua
systemd_user { "mihomo.service", "bsu-login.timer", "napcat.service" }
```

```text
dots sync 做的两件事
════════════════════

① 链接（来自 tree/ 镜像，不是本 API）：
   ~/.config/systemd/user/mihomo.service ──→ tree/home.linux/.config/systemd/user/mihomo.service

② enable（本 API，等价于手敲 systemctl --user enable mihomo.service）：
   ~/.config/systemd/user/default.target.wants/      ← systemctl 创建的真实目录
   └── mihomo.service → ../mihomo.service               （不入仓库：见 granularity 的 ignore）

仓库里只记「应该 enable 什么」这个声明；.wants/ 软链是机器状态，留在本机。
```

> 注意：被 timer 触发、没有 `[Install]`/`WantedBy` 的 service（如 `bsu-login.service`）不要写进来——enable 它没有意义，挂载它的是 `bsu-login.timer`。

### `scripts(spec)` — 脚本聚合选项

`scripts/{common,linux,macos}/` 的脚本在 sync 时聚合成软链落进 `.gen/scripts/`（已在 PATH，经 `~/.config/dots/env.zsh`）。**子目录默认保持目录形态**（整目录链）；想递归拍平、让里面的脚本直接进 PATH 的子目录才写 `ignore_tree`：

```lua
scripts { ignore_tree = { "snippets" } }   -- 仅示例：当前 dots.lua 无需任何 scripts 声明
```

聚合规则（`dots-core/src/scripts.rs`）：顶层文件直接链；子目录默认整目录一根链（树形保住，如 `hypr/`）；**`ignore_tree` 列出的子目录递归拍平**——里面的脚本全部提到 `.gen/scripts/` 顶层。重名记冲突（`dots doctor` 报告）。

```text
仓库侧（脚本源，按平台分层）              .gen/scripts/（聚合产物，进 PATH）
══════════════════════════              ═══════════════════════════════
scripts/
├── common/
│   └── bsu_login.py        ─────────→  bsu_login.py ────→ scripts/common/bsu_login.py
├── linux/
│   ├── media_volume.sh     ─────────→  media_volume.sh ─→ scripts/linux/media_volume.sh
│   ├── plasma              ─────────→  plasma ──────────→ scripts/linux/plasma
│   ├── hypr/               ─────────→  hypr ────────────→ scripts/linux/hypr
│   │   ├── focus_mode.sh               │ ★ 默认行为：整目录一根链、树形保住，
│   │   ├── opacity_toggle.sh           │   无需任何声明；键位里用
│   │   └── screen_effects.sh           │   $DOTS_SCRIPTS/hypr/focus_mode.sh 引用
│   └── snippets/（假设，列入 ignore_tree）
│       └── fixup.sh        ─────────→  fixup.sh ────────→ scripts/linux/snippets/fixup.sh
│                                       ↑ 递归拍平：目录层级消失，脚本提到顶层直接敲名字
└── macos/
    └── （Linux 上不聚合 macos/；macOS 上聚合 common/ + macos/）
```

多次调用 `scripts{}` 是**累加**语义（`extend`），不是覆盖。

何时需要 `ignore_tree`：子目录只是分类用的组织手段、但你希望里面的脚本不带目录前缀直接敲名字。默认保树适合脚本互相引用（`source ./lib.sh`）或外部配置按目录引用（Hyprland 键位写 `$DOTS_SCRIPTS/hypr/xxx.sh`）的常见情况。

> 迁移注：旧字段 `keep_tree` 已移除（语义反转为默认保树），写它会在声明期报错并提示改用 `ignore_tree`。

### `on(hooks)` — 生命周期钩子

声明式 API 都盖不住时的逃生舱。表形式调用（与 `hosts{}` 同构）：phase 做 key，value 是单个函数或函数数组。闭包在声明期只登记不执行，effect 期按阶段触发：

```text
dots sync 时间线（cmd/sync.rs 的真实顺序）
════════════════
eval dots.lua（声明期，闭包只登记）
  │
  ├─ ① on{ pre_sync }              ←─ 全局钩子
  ├─ ② host 激活
  │    ├─ 命中 → 执行 hosts 块闭包（vars/link 在此登记）
  │    │         未命中且表非空 → 打印骨架 + 硬报错
  │    └─ on{ on_host_activate }   ←─ 全局钩子（仅命中时，紧随块闭包）
  ├─ ③ 收集期望链接（层镜像 + distribute + scripts + link()）
  │    └─ 条目级 pre               ←─ spec.pre：返回 false → 条目整体剔除（⊘）
  ├─ ④ resolve → Plan + execute（建链 / 备份 / 重建 / 容器转换）
  ├─ ⑤ 条目级 post                 ←─ spec.post（仅未被阻止的条目）
  ├─ ⑥ .inject 模板渲染
  ├─ ⑦ on{ post_link }             ←─ 全局钩子（链接与 .inject 已就绪）
  ├─ ⑧ env.zsh / zshrc stub
  ├─ ⑨ systemd --user enable
  ├─ ⑩ on{ post_sync }             ←─ 全局钩子（一切就绪后）
  └─ ⑪ 保存 state.json
```

```lua
on {
  post_sync = function()
    dots.run_once("tpm", "git clone https://github.com/tmux-plugins/tpm ~/.tmux/plugins/tpm")
  end,
}

-- 同 phase 多钩子：value 用函数数组（按下标序执行）
on {
  pre_sync = function() ... end,
  post_sync = {
    function() dots.json.merge(...) end,
    function() dots.run_once(...) end,
  },
}
```

| phase | 触发时机 | 典型用途 |
| --- | --- | --- |
| `"pre_sync"` | 链接落盘前（一切之前） | 准备目录、迁移旧布局 |
| `"on_host_activate"` | hosts 块闭包执行完后（仅命中时） | 跨 host 通用的「命中后」逻辑 |
| `"post_link"` | 链接建好、`.inject` 渲染完之后 | 依赖链接存在的操作 |
| `"post_sync"` | 一切就绪后：链接 / inject / env.zsh / systemd enable 都完成（仅台账保存在其后） | 装外部依赖、打补丁、收尾杂活 |

- 未知 phase key → **声明期即报错**（`未知生命周期点：…`）；value 不是函数/函数数组 → 同样声明期报错。
- 钩子体内可用 §3 的全部写原语；沙箱依旧生效（没有 `io`/`os`），副作用只能走原语，这保证了幂等与可审计。
- 同一 phase 多个钩子按登记序执行：数组内按下标序，跨多次 `on{}` 调用按调用序。
- 全局钩子的返回值被忽略——「阻止」能力是条目级 pre（见 granularity / distribute）独有的。

### `hosts(blocks)` — per-host 配置块

机器差异（背光设备名、显示器布局）按 hostname 分发：

```lua
hosts {
  ["wanger-arch-16p"] = function()
    vars { backlight = "amdgpu_bl1", ddc_index = "1" }
    link("hosts/files/wanger-arch-16p/monitors.conf", "~/.config/hypr/monitors.conf")
  end,
  ["macbook"] = function()
    vars { backlight = "intel_backlight" }
  end,
}
```

```text
                    ┌─ hostname == "wanger-arch-16p" ──→ 执行该闭包（vars + link 生效）
dots sync ──读取──→ ┤  hostname == "macbook" ──────────→ 执行 macbook 闭包
                    └─ hostname == 其他 ───────────────→ ⚠ 打印骨架 + warn，继续链通用项
                                                           （per-host 增量跳过，非致命）
```

行为细节（`cmd/sync.rs`）：

- 只有匹配当前 hostname 的闭包会执行（effect 期）。
- **当前机器未命中且表非空 → 打印骨架建议 + warn，但不致命**：照常链接通用配置，仅跳过 per-host 增量。装机的核心是链配置，不能押在主机名探测/登记上。（曾经是硬报错，已改）
- `dots doctor` 也检查 hostname 覆盖（`cmd/doctor.rs:31`）。

#### 主机名解析与新机 onboarding

`hosts{}` 的匹配 key = `hosts::current()`，按优先级解析（前者命中即用，空白跳过）：

1. `$DOTS_HOST` 环境变量——显式覆盖，便于测试/临时切换。
2. `~/.config/dots/host`——**别名文件**（机器本地，不入 git）。
3. `/etc/hostname` → 4. `$HOSTNAME` → 5. `"unknown"`。

**B 方案（别名 + 本地覆盖）**：`dots bootstrap` 在**未登记主机 + 交互终端**时跑一次 host 引导
（`onboard.rs`）：问别名 + 工具链组 → 写 `~/.config/dots/host=<别名>` + 把 host 块**插进 dots.lua**
（现有 `hosts({` 行下方，否则文件尾追加）。于是别名进 git、真实主机名/内网名不进；`current()` 靠本地
文件命中别名块。非交互（CI/`curl|sh`）或回车跳过 → 不写，靠上面的「未命中非致命」兜底。

> 这是 CLI 唯一会写 `dots.lua` 的路径；其余一切仍人手编辑。

---

## 2. per-host API（仅 `hosts{}` 块 / 钩子内可用）

这些函数在 effect 期才被注入 Lua 环境——**在顶层调用会报 `nil` 调用错误**，这是两阶段模型的直接体现。

### `vars(tbl)` — 设置注入变量

```lua
vars { backlight = "amdgpu_bl1", ddc_index = "1" }
```

写入 per-host 变量表，供 `.inject` 模板引用。键值都是 string。多次调用合并。数据流全景（示意——机制已实现，当前 `tree/` 里还没有 `.inject` 文件）：

```text
dots.lua                          .inject 模板（仓库里）                       渲染产物（本机）
════════                          ════════════════════                       ══════════════
hosts {                           tree/home.linux/.config/systemd/user/
  ["wanger-arch-16p"] =           foo.service.inject:
    function()                    ┌───────────────────────────────────────┐
      vars {                      │ [Service]                             │   .gen/injected/…/foo.service
        backlight = "amdgpu_bl1", │ ExecStart={{ SCRIPTS }}/foo.sh        │   ┌─────────────────────────────────────────────┐
      }            ──────────────→ │ Environment=DEV={{ host.backlight }}  │──→ │ ExecStart=/home/w/dotfiles/.gen/scripts/foo.sh│
    end,                          │ Environment=PASS={{ secret.foo_pass }}│   │ Environment=DEV=amdgpu_bl1                  │
}                                 └───────────────────────────────────────┘   │ Environment=PASS=hunter2                    │
dots secret set foo_pass ──────────────────────┘                              └─────────────────────────────────────────────┘
                                                                                            │
                                                       ~/.config/systemd/user/foo.service ──链接──→ .gen/injected/…
                                                       （目标名 = 去掉 .inject 后缀，映射回 $HOME 侧）
```

`.inject` 模板的完整上下文（`inject.rs:38-43`）：

| 变量 | 内容 |
| --- | --- |
| `{{ DOTFILES }}` | 仓库根绝对路径（注意大写） |
| `{{ SCRIPTS }}` | `.gen/scripts/` 绝对路径（大写） |
| `{{ host.<key> }}` | `vars{}` 设置的 per-host 变量 |
| `{{ secret.<key> }}` | `dots secret set` 存的 age 解密值 |

渲染是 **strict 模式**：引用了不存在的变量直接报错（这也是 `dots doctor` 「未解析变量」检查的来源）。可用 minijinja 的 `default` 过滤器兜底：`{{ host.nope | default('fallback') }}`。

### `link(src, target)` — 追加专属链接

```lua
link("hosts/files/wanger-arch-16p/monitors.conf", "~/.config/hypr/monitors.conf")
```

| 参数 | 说明 |
| --- | --- |
| `src` | **相对仓库根**的路径（不是相对 `tree/`） |
| `target` | $HOME 侧目标，可用 `~` |

```text
仓库侧（per-host 资产，不在 tree/ 镜像里）      $HOME 侧（仅在该 host 上）
══════════════════════════════════          ════════════════════════
hosts/files/wanger-arch-16p/
└── monitors.conf  ←───────────────────────  ~/.config/hypr/monitors.conf
                                              （wanger-arch-16p 之外的机器不会有这条链）
```

追加进期望链接集合，与镜像产生的链接走同一套 resolve/execute（同样有备份、漂移检测、state.json 记账）。

为什么不用 `vars` 渲染显示器坐标而是整文件 per-host：hostname 失配时**缺文件会报错**，绝不静默渲出别台机器的显示器布局。

### `toolchains(spec)` — 圈定 bootstrap 工具链范围

```lua
toolchains({ only = { "core" } })        -- 白名单：只装 core 组（服务器典型）
toolchains({ skip = { "ai", "js" } })    -- 黑名单：除 ai/js 都装
```

| 字段 | 说明 |
| --- | --- |
| `only` | 白名单：只装列出的组（与 `skip` 互斥，二选一必填） |
| `skip` | 黑名单：列出的组不装 |

组名 = `packages/toolchains.toml` 的 `[节头]`；host 块不声明 = 全装（旧行为）。引用了清单中不存在的组，bootstrap 时警告（防拼写错静默漏装）。

```text
packages/toolchains.toml             dots.lua                          dots bootstrap @ VM-0-6-ubuntu
════════════════════════             ════════                          ═════════════════════════════
[core]                               hosts {                           ✔ 安装 uv…
uv = "curl …"            ──┐           ["VM-0-6-ubuntu"] =             ✔ 安装 starship…
starship = "cargo …"       ├─ 装 ──→     function()                    ✔ 安装 zoxide…
zoxide = "cargo …"       ──┘             toolchains({                  ✔ 安装 dust…
                                           only = { "core" },
[dev]                    ──┐             })                            （dev/ai/js 不出现）
cargo-nextest / prek / …   │           end,
[ai]                       ├─ 跳过    }
claude                     │
[js]                       │
pnpm / nvm!              ──┘
```

执行时序：bootstrap 在装工具链**之前**单独 eval dots.lua 并以 dry-run 激活当前 host 块（只收集声明、不落盘副作用），读到 filter 后过滤清单；收尾的 sync 会再次正常激活 host 块——`vars`/`link` 等真正的副作用属于 sync 阶段。

---

## 3. `dots` 全局表

### 只读上下文（声明期就可用）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `dots.host` | string | 当前主机名 |
| `dots.os` | `"linux"` \| `"macos"` | 当前平台 |
| `dots.home` | string | `$HOME` 绝对路径 |
| `dots.repo` | string | 仓库根绝对路径 |

顶层就能用，比如做平台条件：

```lua
if dots.os == "linux" then
  systemd_user { "mihomo.service" }
end
```

（不过平台差异优先用 `tree/home.linux/` 层表达，这里只是兜底。）

### 写原语（effect 期注入，仅闭包内可用）

写原语共享统一保证（`primitives.rs`）：

- **原子写**：temp 文件 + rename，永远不会留半截文件。
- **无差异不写**：内容相同直接跳过，mtime 不变（不惊扰 watch 这些文件的程序）。
- **dry-run 感知**：`--dry-run` 时只打印意图（`⇄ will merge …`），不碰盘。
- **读-改-写型记 ownership**：写过的键登记进 `.dots/state.json`，`dots doctor` 据此检测「我写的键被别人删了」这类漂移。

#### `dots.run_once(key, cmd) → boolean`

幂等执行一次性命令。`key` 记账到 state.json，已执行过则跳过：

```lua
on {
  post_sync = function()
    dots.run_once("import-zoxide", "zoxide import --from z ~/.z")
  end,
}
```

```text
第一次 sync                              之后每次 sync
═══════════                              ═════════════
state.json: { "run_once": [] }           state.json: { "run_once": ["import-zoxide"] }
       │                                        │
       ├─ key 不在账上                          ├─ key 已在账上
       ├─ sh -c "zoxide import …"  执行         └─ 跳过，返回 false
       ├─ 退出码 0 → 记账，返回 true
       └─ 退出码非 0 → 报错，不记账（下次 sync 重试）
```

适用：装插件管理器、一次性数据迁移。不适用：需要跟随配置变化重跑的东西（那该是普通脚本或钩子逻辑）。

dry-run 时命令不执行（只在会话内存里标记，state 不落盘），所以下次真实 sync 仍会执行——预览不会「消耗」掉一次性命令。

#### `dots.run(cmd) → { code, stdout, stderr, ok }`

每次 sync 都执行的命令（与 `run_once` 互补：不记账、不跳过），**捕获输出富返回**——分支逻辑留在 Lua 里写，不必塞进 shell 一行流。非零退出留一行告警但不致命，分支交给调用方。

```lua
on {
  post_sync = function()
    local probe = dots.run("systemctl --user is-active mihomo.service")
    if not probe.ok then
      dots.run("systemctl --user restart mihomo.service")
    end
  end,
}
```

```text
每次 sync
═════════
sh -c "<cmd>"，stdout/stderr 全捕获
  ├─ 退出 0  → 返回 { code=0, stdout="…", stderr="…", ok=true }
  └─ 退出 N  → 告警「dots.run 退出码 N」后照常返回
               { code=N, stdout="…", stderr="…", ok=false }，sync 不中断
```

- `code`：退出码（被信号杀死等无退出码 → `-1`）。
- 适用：探测系统状态、刷新缓存这类「每次都该对账」的命令。编译派生二进制用专门的 [`dots.cargo.build`](#dotscargobuilddir-bin--pathnil-err)。
- dry-run 时不执行，直接返回 `{ code=0, stdout="", stderr="", ok=true }`。

#### `dots.cargo.build(dir, bin) → path|nil, err?`

release 编译 `dir` 下 workspace 的某个 bin，返回**产物绝对路径**。路径从 cargo `--message-format=json` 的消息流里取，不猜 `target/` 布局——`CARGO_TARGET_DIR` 重定向也照常工作。

```lua
local bin, err = dots.cargo.build(dots.repo .. "/cli", "cc-hook")
```

```text
输入                                      返回值
════                                      ══════
<repo>/cli/                               "/…/cli/target/release/cc-hook"  ← 绝对路径
├── Cargo.toml      ──cargo build──→        ├─ 编译失败 → nil, "<stderr 摘要>"（告警留痕，sync 不中断）
└── crates/cc-hooks/…                       └─ dry-run  → nil, "dry-run"（不编译，下游自然短路）
```

- 增量编译：无代码改动时秒回。
- 产物在 `target/` 下，**会被 `cargo clean` 清掉**——别软链它，配合 `dots.file.install` 复制到稳定位置。

#### `dots.file.install(src, dest)`

原子安装文件（复制而非软链）：内容无差异**跳写**；否则 temp + rename 替换——rename 只改目录项，正在运行的旧二进制不受影响（免 ETXTBSY），权限位与源一致。

```lua
on {
  post_sync = function()
    local bin = dots.cargo.build(dots.repo .. "/cli", "cc-hook")
    if bin then
      dots.file.install(bin, dots.home .. "/.claude/hooks/cc-hook")
    end
  end,
}
```

```text
源（易失区，cargo clean 会清空）          落点 before              落点 after
════════════════════════════════          ═══════════              ══════════
<repo>/cli/target/release/                ~/.claude/hooks/         ~/.claude/hooks/
└── cc-hook  ────────复制────────→        └── pretool.toml         ├── pretool.toml
                                                                   └── cc-hook  ★ 真实文件非软链，
                                                                                  源被清也照常工作
```

- 为什么不软链：软链指向 `target/` 时，`cargo clean` 一跑链接就断——对 fail-open 的 hook 来说是**静默失效**（守卫消失和守卫放行外观相同）。
- dest 支持 `~`；幂等：bin 没变第二次 sync 不写盘（mtime/inode 不动）。

#### `dots.json.merge(path, tbl)`

读-改-写 JSON：把 `tbl` 深合并进目标文件，**保留其余键**。

```lua
on {
  post_sync = function()
    dots.json.merge(dots.home .. "/.claude/settings.json", {
      statusLine = { type = "command", command = "starship-statusline" },
    })
  end,
}
```

```text
目标文件 before                  overlay（你给的 tbl）            目标文件 after
═══════════════                  ════════════════════            ══════════════
{                                {                                {
  "model": "opus",        ＋       "statusLine": {         ＝      "model": "opus",          ← 别人的键，保留
  "permissions": { … }             "type": "command",              "permissions": { … },     ← 保留
}                                    "command": "…"                  "statusLine": {           ← 你的键，写入
                                   }                                  "type": "command",
                                 }                                    "command": "…"
                                                                    }
                                                                  }
```

- 合并语义：object 递归合并，**数组和标量整体覆盖**（`merge_json`，`primitives.rs:46-60`）。
- 目标不存在或为空 → 当 `{}` 处理（即「创建」）。
- 输出 pretty-print JSON。
- `tbl` 的**顶层键**（上例的 `statusLine`）登记 ownership，doctor 之后会盯着它。

解决的问题：软链接要求「仓库独占整个文件」，但有的 JSON 被软件自己读写（不能链），你又只关心其中几个键——merge 让你只管你那几个键。

#### `dots.json.set(path, keypath, value)`

按点路径设单个值，是 merge 的便捷形式：

```lua
dots.json.set(dots.home .. "/.config/tool/cfg.json", "editor.fontSize", 14)
```

```text
keypath 展开（nest_keypath）                 等价的 merge 调用
═══════════════════════════                 ════════════════
"editor.fontSize" = 14        ──────→        { editor = { fontSize = 14 } }
     │      │
     └──────┴── 按 "." 切分，从右往左包成嵌套 object
```

#### `dots.json.decode(text) → table|nil, err?`

JSON 文本解析成 Lua 表（纯函数，不碰盘）。给 `dots.run` 捕获的命令输出做结构化消费——别再用 `string.match` 抠字段：

```lua
local obj, err = dots.json.decode('{"name":"cc-hook","executable":null,"deps":[1,2]}')
```

```text
JSON 输入                                 Lua 表产物
═════════                                 ═════════
{ "name": "cc-hook",                      obj.name        → "cc-hook"
  "executable": null,          ──────→    obj.executable  → nil   ★ null 映射为 nil，
  "deps": [1, 2] }                        obj.deps[2]     → 2        if obj.x then 判断不被骗

"not json"                     ──────→    nil, "expected value at line 1 …"（双返回报错）
```

注意它解析的是**单个 JSON 文档**——JSON Lines 输出（每行一个文档）要 `gmatch("[^\n]+")` 逐行喂。

#### `dots.file.ensure_block(path, marker, content)`

在文本文件里维护一段 managed block，幂等替换：

```lua
dots.file.ensure_block(dots.home .. "/.bashrc", "dots-env",
  'source "$HOME/.config/dots/env.zsh"')
```

```text
目标文件 before                          目标文件 after（第 1 次和第 N 次结果相同）
═══════════════                          ═══════════════════════════════════════
# my old bashrc                          # my old bashrc
alias ll='ls -l'                         alias ll='ls -l'
                                         
                                         # >>> dots:dots-env >>>          ← marker 包裹的受管区间
                                         source "$HOME/.config/dots/env.zsh"
                                         # <<< dots:dots-env <<<
                                         
（content 改了再 sync：只有区间内被替换，前后内容原样不动）
```

- 文件里已有同 marker 区块 → 原位替换（前后内容不动）；没有 → 追加到末尾；文件不存在 → 创建。
- 反复 sync 不会越插越多——这就是它和「往文件 append」的区别。
- 注释前缀是写死的 `#`（`primitives.rs:229-232`），适合 shell/conf 类文件，不适合 `//` 注释的语言。

---

## 4. 配方

**某工具往配置目录写缓存，污染仓库：**

```lua
granularity("home/.config/<tool>", { mode = "file", ignore = { "cache" } })
```

**新接入一个想共享 skills 的 AI 工具：**

```lua
distribute("skills", {
  src = "tree/home/.agent/skills",
  to = { "~/.claude/skills", "~/.codex/skills", "~/.copilot/skills", "~/.newtool/skills" },  -- ← 加一行即可
  mode = "children",
})
```

**新机器第一次 sync 报「未在 hosts{} 覆盖」：**

照 sync 打印的骨架在 `hosts{}` 里加一个块（空的也行），再 sync。

**给被软件托管的 JSON 配置打补丁：**

```lua
on {
  post_sync = function()
    dots.json.merge(dots.home .. "/.claude/settings.json", { model = "opus" })
  end,
}
```

**装一次性的外部依赖：**

```lua
on {
  post_sync = function()
    dots.run_once("rustup-component", "rustup component add rust-analyzer")
  end,
}
```

**保持自编译二进制新鲜（且不怕 `cargo clean`）：**

```lua
on {
  post_sync = function()
    local bin = dots.cargo.build(dots.repo .. "/cli", "cc-hook")
    if bin then dots.file.install(bin, dots.home .. "/.claude/hooks/cc-hook") end
  end,
}
```

## 5. 速查表

| API | 阶段 | 作用域 | 一句话 |
| --- | --- | --- | --- |
| `granularity(path, spec)` | 声明 | 顶层 | 覆盖链接粒度 |
| `distribute(name, spec)` | 声明 | 顶层 | 一源多落点 |
| `root(name, spec)` | 声明 | 顶层 | $HOME 外的映射层 |
| `systemd_user{…}` | 声明 | 顶层 | sync 时 enable 单元 |
| `scripts{ignore_tree=…}` | 声明 | 顶层 | 脚本聚合：列出的子目录拍平（默认保树形） |
| `on{ phase = fn \| {fn,…} }` | 声明（执行在 effect） | 顶层 | 全局生命周期钩子 |
| `spec.pre` / `spec.post` | 声明（执行在 effect） | granularity/distribute 的 spec 内 | 条目级钩子；pre 返回 false 阻止该条目 |
| `hosts{…}` | 声明（执行在 effect） | 顶层 | per-host 分发 |
| `vars{…}` | effect | hosts 块/钩子内 | 注入变量（`.inject` 用） |
| `link(src, target)` | effect | hosts 块/钩子内 | 追加专属链接 |
| `dots.host/os/home/repo` | 都可| 都可 | 任意 | 只读上下文 |
| `dots.run_once(key, cmd)` | effect | 闭包内 | 幂等一次性命令 |
| `dots.run(cmd)` | effect | 闭包内 | 每次执行，富返回 `{code,stdout,stderr,ok}` |
| `dots.cargo.build(dir, bin)` | effect | 闭包内 | release 编译 bin，返回产物绝对路径 |
| `dots.json.merge(path, tbl)` | effect | 闭包内 | JSON 深合并（保留其余键） |
| `dots.json.set(path, kp, v)` | effect | 闭包内 | JSON 按点路径设值 |
| `dots.json.decode(text)` | 都可 | 任意 | JSON 文本 → Lua 表（null→nil） |
| `dots.file.ensure_block(p, m, c)` | effect | 闭包内 | 文本 managed block |
| `dots.file.install(src, dest)` | effect | 闭包内 | 原子复制安装（免 ETXTBSY、保留权限位） |

实现入口：DSL 注册 `cli/crates/dots/src/lua/api.rs`，写原语 `cli/crates/dots/src/lua/primitives.rs`，沙箱与上下文 `cli/crates/dots/src/lua/eval.rs`，类型标注 `cli/lua-api/dots.meta.lua`。
