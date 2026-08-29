# dots.lua API

`dots.lua` 只声明持续状态和 mapping。它在无 `io`、`os`、`require`、`load` 的 Lua 沙箱中求值，不能直接执行命令或写文件。

## 收敛模型

每次 `sync` 比较三份事实：

- **Desired Set**：当前 `tree/`、`dots.lua` 与内建生成规则要求存在的 Resource。
- **Applied Inventory**：`.dots/state.json` 中上一次成功由 dots 拥有的 Resource。
- **Observed State**：Resource 的 Ownership Surface 当前真实状态。

一个 Declaration 从 Desired Set 消失后，Resource 进入 retirement：

- Observed 仍匹配 Applied：安全删除。
- Observed 已不存在：只从 inventory 移除。
- Observed 被修改：报告 Drift，保留实际内容和 inventory。

未拥有的 target 已与 Desired 完全一致时，sync 不重写 target，直接记入 inventory。未拥有且内容不同则是 Collision，普通 sync 不接管。

`dots status`、`dots sync --dry-run` 与真实 `dots sync` 使用同一 Plan。dry-run 不修改受管位置和 inventory；为了精确判断 Cargo binary 内容，它会执行编译并允许 Cargo 更新 `target/` cache。

## 路径规则

- Resource `source` 和 `manifest`：相对路径以仓库根为基准；也接受绝对路径和 `~`。
- Resource `target`：必须是绝对路径或 `~` 路径。
- identity 从 Ownership Surface 推导，不填写单独 id。
- `enabled` 缺省为 `true`。设为 `false` 后该 Resource 不进入 Desired Set；之前已应用的同一 Resource 会进入 retirement。

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
    to = { "~/.claude/skills", "~/.codex/skills" },
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

## 显式 Resource

所有方法都在 `dots.resource` 下，接收一个 typed table。共同可选字段：

| 字段 | 类型 | 缺省 | 行为 |
| --- | --- | --- | --- |
| `enabled` | `boolean` | `true` | `false` 时不进入 Desired Set |

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
        manifest = "cli/Cargo.toml",
        binary = "cc-hook",
    },
    target = "~/.claude/hooks/cc-hook",
}
```

workspace source 在 planning 中执行：

```bash
cargo build --release --bin <binary> --message-format=json --manifest-path <manifest>
```

dots 使用 Cargo 报告的 `executable` path，不猜 `target/` 位置；然后把 artifact 作为普通文件 Resource 收敛到 `target`。

crates.io package 只声明 source：

```lua
dots.resource.cargo_binary {
    source = "ripgrep",
}
```

字符串 source 表示 crates.io package；table source 表示仓库内 workspace binary。crates.io derivation 固定执行 `cargo install --locked <package>`，不接受 `version`、`binary`、`target`、额外 Cargo 参数、git source 或 shell command。Cargo 决定默认版本并安装 package 提供的全部 bin；Dots 读取实际文件名，分别归一为 `~/.cargo/bin/<文件名>` 的普通文件 Resource。例如 `ripgrep` 产生 `~/.cargo/bin/rg`，无需重复声明映射。

derivation 产物写入 `.gen/cargo-install/<source-digest>/`；该目录只是可重建 cache，不进入 Applied Inventory，也不随 Declaration retirement 删除。声明删除后，该 package 生成的每个 target binary 都分别进入正常 retirement。

两种 source 的最终 target 使用相同的文件 lifecycle：内容或权限变化产生 Update，target 被外部修改产生 Drift，Declaration 删除时只删除仍匹配 Applied 状态的 binary。planning、status 与 dry-run 都可能执行 Cargo derivation 并写入 Cargo target 或 `.gen` cache，但不会修改最终 target。

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
local node_modules = dots.repo .. "/pi-ext/node_modules"
dots.resource.symlink {
    source = node_modules,
    target = "~/.pi/agent/node_modules",
    enabled = dots.path.exists(node_modules),
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

需要自动删除的持续结果必须建模为 Resource；一次性命令在 `dots sync` 之外明确执行。
