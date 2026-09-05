# dots.lua API

`dots.lua` 声明持续状态、mapping、sync lifecycle hook 和 Cargo binary 安装意图。它在无 `io`、`os`、`require`、`load` 的 Lua 沙箱中求值；命令只能通过 typed hook 交给 dots 执行。

## 收敛模型

每次 `sync` 比较三份事实：

- **Desired Set**：当前 `tree/`、参与 sync 的 `dots.lua` 声明与内建生成规则要求存在的 Resource。
- **Applied Inventory**：`.dots/state.json` 中上一次成功由 dots 拥有的 Resource。
- **Observed State**：Resource 的 Ownership Surface 当前真实状态。

一个 Declaration 从 Desired Set 消失后，Resource 进入 retirement：

- Observed 仍匹配 Applied：安全删除。
- Observed 已不存在：只从 inventory 移除。
- Observed 被修改：报告 Drift，保留实际内容和 inventory。

未拥有的 target 已与 Desired 完全一致时，sync 不重写 target，直接记入 inventory。未拥有且内容不同则是 Collision，普通 sync 不接管。

`dots status`、`dots sync --dry-run` 与真实 `dots sync` 使用同一 Resource Plan。只有真实 sync 在读取机器状态和 planning 前执行 lifecycle hook；dry-run 显示 hook 但不启动进程，status 完全忽略 hook。dry-run 不修改受管位置和 inventory。这三个命令都忽略 Cargo binary declaration，不会因此执行 Cargo 或读取已安装 binary 的内容。

## 路径规则

- Resource `source` 与 Cargo `path`：相对路径以仓库根为基准；也接受绝对路径和 `~`。
- Resource `target` 与 Cargo `root`：必须是绝对路径或 `~` 路径。
- identity 从 Ownership Surface 推导，不填写单独 id。
- `enabled` 缺省为 `true`。sync Resource 设为 `false` 后退出 Desired Set 并进入正常 retirement；Cargo binary 设为 `false` 后不被 `dots install` 执行，已经安装的 binary 保持不变。

## Mapping declaration

### `granularity(path, spec)`

覆盖 `tree/` 中一个路径的链接粒度：

```lua
granularity("home/.config/opencode", {
    mode = "file",
    ignore = { "node_modules", "bun.lock" },
})
```

字段：

| 字段 | 类型 | 行为 |
| --- | --- | --- |
| `mode` | `"dir" | "children" | "file"` | 整目录 link、逐子项 link、递归逐文件 link；缺省为 `dir` |
| `ignore` | `string[]` | 下钻时跳过这些直接子项名 |

`granularity` 不接受 `pre` 或 `post`；传入任一字段都会报错。

### `distribute(name, spec)`

把同一 source 分发到多个工具目录：

```lua
distribute("skills", {
    src = "tree/home/.agents/skills",
    to = { "~/.codex/skills", "~/.kimi/skills" },
    mode = "children",
})
```

字段：

| 字段 | 类型 | 行为 |
| --- | --- | --- |
| `src` | `string` | 仓库内 source |
| `to` | `string[]` | 目标列表 |
| `mode` | `"dir" | "children" | "file"` | 目标粒度 |

目标的父工具目录不存在时，该 target 不进入 Desired Set；目录存在的 sync 会创建它。Applied target 的父目录消失时，inventory 按正常 Observed/retirement 规则处理。

### `root(name, spec)`

为 `tree/<name>` 声明 `$HOME` 外的目标根：

```lua
root("appsupport", {
    path = "~/Library/Application Support",
    os = "macos",
})
```

### `scripts(spec)`

`scripts/common` 与 `scripts/<os>` 聚合到 `.gen/scripts/`。子目录默认整目录 link；`ignore_tree` 列出的子目录递归拍平：

```lua
scripts { ignore_tree = { "legacy-flat" } }
```

同名脚本产生 ownership conflict，整个 sync 在外部写入前失败。

## Lifecycle hook

### `dots.hook.before_sync(spec)`

在真实机器状态读取和 Resource planning 前运行一条具名程序：

```lua
dots.hook.before_sync {
    name = "install Pi dependencies",
    cwd = dots.repo .. "/pi",
    program = "pnpm",
    args = { "install", "--frozen-lockfile" },
}
```

| 字段 | 类型 | 缺省 | 行为 |
| --- | --- | --- | --- |
| `name` | `string` | — | 日志和失败诊断使用的名称 |
| `cwd` | `string` | — | 启动程序时使用的工作目录 |
| `program` | `string` | — | 由当前 `PATH` 或明确路径解析的程序 |
| `args` | `string[]` | `{}` | 按原样传给程序的参数 |
| `enabled` | `boolean` | `true` | `false` 时不声明该 hook |

同一阶段按声明顺序执行。任一 hook 无法启动或返回非零状态时，sync 立即失败，不读取真实机器状态、不生成 Plan，也不修改 Resource 或 Applied Inventory。hook 每次真实 sync 都运行，不持有 Resource ownership，也不提供 run-once 状态。

`dots sync --dry-run` 输出 `would run before_sync hook`；`dots status` 不执行或显示 hook。

## `dots.resource` declaration

所有方法都在 `dots.resource` 下，接收一个 typed table。共同可选字段：

| 字段 | 类型 | 缺省 | 行为 |
| --- | --- | --- | --- |
| `enabled` | `boolean` | `true` | `false` 时消费该 declaration 的命令忽略本项 |

sync Resource 被禁用后退出 Desired Set 并正常 retirement；Cargo binary 被禁用后不由 `dots install` 执行，已经安装的 binary 保持不变。

### `dots.resource.symlink(spec)`

```lua
dots.resource.symlink {
    source = dots.home .. "/.config/opencode/node_modules",
    target = dots.repo .. "/tree/home/.config/opencode/node_modules",
}
```

必填 `source`、`target`。source 是目录时，该 symlink 同时拥有 target 的后代路径，不能再声明其下的其他 Resource。

### `dots.resource.copied_file(spec)`

```lua
dots.resource.copied_file {
    source = "payload/tool-config",
    target = "~/.config/tool/config",
}
```

目标是普通文件，不是 symlink。sync 按完整内容和 Unix permission bits 判断更新、Drift 与安全删除。

### `dots.resource.cargo_binary(spec)`

```lua
dots.resource.cargo_binary {
    source = {
        path = "cli/crates/agent-hooks",
        binary = "agent-hook",
    },
    root = "~/.local",
}
```

workspace declaration 只在 `dots install` 中执行：

```bash
cargo install --locked --path <path> --bin <binary> --root <root>
```

`path` 指向包含目标 package 的目录，`binary` 和 `root` 原样映射为 Cargo 参数。dots 不解析 artifact，也不自行复制 binary。

crates.io package 只声明 source：

```lua
dots.resource.cargo_binary {
    source = "ripgrep",
}
```

字符串 source 表示 crates.io package；table source 表示仓库内 workspace binary。`dots install` 缺省对 crates.io declaration 执行 `cargo install --locked <package>`，使用真实 Cargo home 和 Cargo 自己的安装 metadata 判断安装或升级。Cargo 已记录的旧版本会按 Cargo 自身规则升级；已有但未被 Cargo 记录的同名 binary 仍由 Cargo 报错。dots 不自动追加 `--force`。

package 含有当前平台不会安装的 gated binary、导致 Cargo 每次误判为需要重装时，可以明确选择实际需要的 binary。每个 `binaries` 条目直接映射为一个 `--bin`：

```lua
dots.resource.cargo_binary {
    source = "uv",
    binaries = { "uv", "uvx" },
}
```

API 不接受 version、额外 Cargo 参数、git source 或 shell command。`dots install` 只遍历本次 Manifest 中存在且启用的 declaration。删除或禁用 declaration 不会执行 `cargo uninstall`，也不会删除、校验或记录之前安装的 binary。

### `dots.resource.managed_block(spec)`

```lua
dots.resource.managed_block {
    target = "~/.profile",
    marker = "tool-env",
    content = "export TOOL_HOME=\"$HOME/.tool\"",
}
```

生成格式：

```text
# >>> dots:tool-env >>>
export TOOL_HOME="$HOME/.tool"
# <<< dots:tool-env <<<
```

Ownership 仅覆盖 marker 区间，文件其他内容始终保留。以下情况是 Drift：

- block 内容偏离 Applied 状态；
- begin/end marker 缺失；
- marker 重复或次序错误；
- 同一文件中两个不同 marker block 的实际区间重叠。

managed block 发生 Drift 后不会自动修复。恢复实际 block 到 Applied 或 Desired 内容后再 sync；需要保留实际 block 并放弃 ownership 时使用 `dots forget`。Declaration 仍存在时，下次 sync 会按未拥有状态重新判断该 block。

### `dots.resource.systemd_user_unit(spec)`

```lua
dots.resource.systemd_user_unit {
    unit = "mihomo.service",
}
```

Declaration 存在时保持 `systemctl --user enable`；Declaration 删除时，在 Observed 仍匹配 Applied 的前提下执行 disable。enable/disable 失败会保留旧 inventory，供下次 sync 重试。

## 只读查询

### `dots.path.exists(path)`

声明阶段读取路径是否存在，常用于可选 source：

```lua
local optional_runtime = dots.repo .. "/vendor/tool/runtime"
dots.resource.symlink {
    source = optional_runtime,
    target = dots.home .. "/.local/share/tool/runtime",
    enabled = dots.path.exists(optional_runtime),
}
```

当条件从 true 变成 false，Resource 会退出 Desired Set 并正常 retirement，不会遗留旧 symlink。

## Collision、Drift 与 forget

普通 sync 遇到陌生且不同的 target 时报告 Collision，不自动覆盖：

手工移走或删除陌生 target 后再 sync。已拥有的 Resource 偏离 Applied 状态时报告 Drift，同样不自动修复。

需要放弃某项 Applied Resource 的 ownership，同时保留机器上的当前内容时：

```bash
dots forget 'path:/Users/me/.config/tool/config'
```

`forget` 只删除 Applied Inventory 记录，不构建 Plan，也不读取或修改真实对象。它既可用于 Retired Resource，也可用于仍在 Desired Set 的 Resource。Declaration 仍存在时，下次 sync 会把该对象视为未拥有：实际状态与声明一致则重新接管，否则报告 Collision。

## 不支持的 API

以下名称没有 API 或 deprecated alias：

- `on`
- `granularity.pre/post`
- `distribute.pre/post`
- `link`
- `systemd_user`
- `dots.run`
- `dots.run_once`
- `dots.file.*`
- `dots.cargo.build`
- `dots.json.*`

需要自动删除的持续结果必须建模为 sync Resource。每次 sync 在 planning 前必须完成的准备命令使用 `dots.hook.before_sync`；Cargo binary declaration 只由显式运行的 `dots install` 执行，其他一次性命令仍在 `dots sync` 之外明确执行。
