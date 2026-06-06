# dots.lua 钩子体系重设计：on{} 表形式 + 条目级 pre/post

日期：2026-06-06
状态：已确认，待实现

## 1. 背景与动机

现状三个问题：

1. `on(phase, fn)` 是字符串双参签名，与 `hosts{}`/`scripts{}`/`systemd_user{}` 的 table 调用糖风格不统一。
2. `HookPhase::OnHostActivate` 定义了、`parse` 接受、注册不报错，但 `sync.rs` 从不触发——死 phase（文档核查中发现，`sync.rs:51,93-94` 只跑 PreSync/PostLink/PostSync）。
3. 钩子只有全局粒度。想对单个声明条目（如 `granularity("home/.claude", …)`）做链接前后处理，只能写在全局 `on{}` 里靠注释关联——可读性差，且没有「条件跳过该条目」的能力。

迁移成本：`dots.lua` 当前没有任何 `on()` 调用，旧签名可直接删除，唯一存量是 `eval.rs` 一处测试用法。

## 2. 已确认的设计决策

| 分叉 | 决定 |
| --- | --- |
| `on` 调用形式 | 表形式 `on{ phase = fn }`，与 hosts{} 同构；**旧式 `on(phase, fn)` 删除** |
| 同 phase 多钩子 | value 接受单函数或函数数组 `on{ post_sync = { fn1, fn2 } }` |
| `on_host_activate` 死 phase | 补实现：hosts 块闭包执行后触发 |
| 条目级钩子声明 | 内联进 `granularity`/`distribute` 的 spec（`pre = fn, post = fn` 字段） |
| pre 阻止语义 | `return false` 阻止；`nil`（无 return）/`true`/其他值 → 继续 |
| hosts 块内联 pre/post | 不加——块闭包本身是代码，pre/post 逻辑直接写块内开头/末尾 |

## 3. sync 生命周期全景（权威图）

```text
dots sync
═════════
┌─ 声明期（沙箱 eval dots.lua）────────────────────────────────┐
│ granularity/distribute/on/hosts… 只登记进 Manifest            │
│ 所有闭包（全局钩子、条目级 pre/post、hosts 块）只存句柄不执行  │
└───────────────────────────────────────────────────────────────┘
  ↓
┌─ effect 期 ───────────────────────────────────────────────────┐
│ ① on{ pre_sync }                ←─ 全局钩子                    │
│ ② host 激活                                                    │
│    ├─ hostname 匹配 hosts{} 表                                 │
│    │    ├─ 命中   → 执行该块闭包（vars{}/link() 登记，         │
│    │    │           也可直接调 dots.run_once 等写原语）        │
│    │    └─ 未命中 → 表非空：打印骨架 + 硬报错；表空：继续      │
│    └─ on{ on_host_activate }    ←─ 全局钩子，紧随块闭包之后    │
│ ③ 收集期望链接：层镜像 + distribute + scripts + hosts 的 link()│
│ ④ resolve → Plan（每条链接的判定：链/重建/备份/漂移）          │
│ ⑤ 条目级 pre                    ←─ spec.pre                    │
│    └─ return false → 该条目的全部链接从 Plan 剔除，报「⊘ 跳过」│
│ ⑥ execute Plan（建链 / 备份 / 重建 / 容器转换）                │
│ ⑦ 条目级 post                   ←─ spec.post（仅未被阻止的条目）│
│ ⑧ .inject 模板渲染（host.* / secret.*）                        │
│ ⑨ on{ post_link }               ←─ 全局钩子                    │
│ ⑩ on{ post_sync }               ←─ 全局钩子                    │
│ ⑪ env.zsh / zshrc stub 写入                                    │
│ ⑫ systemd --user enable                                        │
│ ⑬ state.json 台账保存                                          │
└───────────────────────────────────────────────────────────────┘
```

分工：**全局 `on{}`** 管 sync 整体节奏（①②⑨⑩），**条目级 pre/post** 跟着声明走、管单个条目的链接前后（⑤⑦），**hosts 块闭包**是 per-host 的可执行代码（②）。

`on_host_activate` 与 hosts 块的关系：它实质是「host 块的全局 post」——跨机器通用的命中后逻辑写它；per-host 逻辑直接写块闭包内。

## 4. API 规格

### 4.1 `on{}` 表形式

```lua
on {
  pre_sync = function() ... end,           -- 单函数
  post_sync = {                            -- 或函数数组（同 phase 多钩子）
    function() ... end,
    function() ... end,
  },
}
```

- phase key：`pre_sync` | `on_host_activate` | `post_link` | `post_sync`。
- 错误（声明期即报）：
  - 未知 phase key →「未知生命周期点：xxx」（沿用现有文案）
  - value 既非 function 也非 table → 类型错误
  - 数组元素混入非 function → 类型错误
- 执行顺序：同 phase 按登记序——数组内按下标序，跨多次 `on{}` 调用按调用序。（table key 遍历序不确定，但不同 phase 按各自时机触发，不受影响。）
- 全局钩子返回值忽略（无阻止能力）。

### 4.2 条目级 pre/post（granularity / distribute 的 spec 字段）

```lua
granularity("home/.claude", {
  mode = "children",
  ignore = { "projects", "todos" },
  pre = function()
    return dots.os == "linux"   -- false → 整条目跳过；nil/true/其他 → 继续
  end,
  post = function()
    dots.run_once("claude-init", "…")
  end,
})

distribute("skills", {
  src = "tree/home/.claude/skills",
  to = { "~/.codex/skills" },
  mode = "children",
  pre = function() ... end,     -- false → 整个分发跳过
  post = function() ... end,
})
```

- **pre**（⑤）：按声明序执行；`return false` → 该条目展开的**全部**链接从 Plan 剔除；执行报告增加「⊘ N 跳过」。
- **post**（⑦）：仅未被阻止的条目，紧跟 execute（在 `.inject` 和全局 post_link 之前），按声明序。
- 闭包无参数（条目路径在声明处可见）；体内可用全部写原语。
- **dry-run**：pre 照跑（保证预览准确；闭包内原语自带 dry-run 感知，安全）；post 不跑（效果未发生）。被阻止条目在 dry-run 输出中显示「⊘ pre 跳过」。
- `root()` 暂不加 pre/post（YAGNI，本身无使用者）。

### 4.3 `on_host_activate` 补实现

`sync.rs` 在 `activate_host` 返回命中后补一行：

```rust
let hit = activate_host(&hostname, &manifest, &handles, &effect)?;
if !hit && !manifest.host_blocks.is_empty() { /* 现有硬报错 */ }
if hit {
    run_phase(HookPhase::OnHostActivate, &manifest, &handles, &effect)?;
}
```

## 5. 实现要点

- **manifest（dots-core）**：`GranularitySpec`/`DistributeSpec` 增加 `pre: Option<ClosureId>`、`post: Option<ClosureId>`（ClosureId 机制照搬现有 hooks）。core 不持有 Lua 闭包，只记序号——架构不变。
- **api.rs（dots bin）**：
  - `register_on` 参数从 `(String, Function)` 改为 `Table`，遍历 pairs，value 按 `mlua::Value` 判型（Function → 单条登记；Table → 按下标序遍历登记；其他 → 报错）。
  - `register_granularity`/`register_distribute` 解析 spec 时提取 `pre`/`post` 闭包存 registry。
- **sync.rs（dots bin）**：
  - ②后补 `run_phase(OnHostActivate)`（命中时）。
  - resolve 与 execute 之间：对带 pre 的声明条目跑闭包，false → 按条目路径前缀从 Plan 剔除相关链接，收集 skipped 列表。
  - execute 后：对未被阻止且带 post 的条目跑闭包。
  - 执行报告行增加跳过计数。
- **Lua 只活在 bin 层**，core 维持纯逻辑可测。

## 6. 测试计划（rust-tdd，先红后绿）

eval.rs / api 单测：

- `on{}` 表形式单函数登记（Manifest hooks 数量与 phase 正确）
- 数组多函数登记（ClosureId 数量与顺序）
- 未知 phase key 报错
- value 类型错（string/number）报错
- 数组内混非函数报错
- granularity/distribute spec 的 pre/post 闭包被登记（且声明期不执行）
- 改写现有 `registers_hook_without_running` 为表形式

sync 集成测：

- `on_host_activate` 命中 host 块时触发、空 hosts 表时不触发
- pre 返回 false → 该条目链接全部不落盘，报告含跳过计数
- pre 返回 nil → 正常链接
- post 仅对未阻止条目执行
- dry-run：pre 执行（可观测其原语意图输出）、post 不执行

## 7. 文档更新清单

- `cli/lua-api/dots.meta.lua`：`on` 改 `@param hooks table<HookPhase, fun()|fun()[]>`（`@alias` 定义 phase 联合）；`GranularitySpec`/`DistributeSpec` 加 `pre`/`post` 字段标注（`fun(): boolean?`）。
- `docs/LUA_API.md`：`on()` 段改表形式签名与示例；phase 表撕掉 ⚠️ 未触发标注；sync 时间线替换为 §3 全景图；granularity/distribute 段补 pre/post 与阻止语义；速查表更新。
- `dots.lua` 无需迁移（无存量 `on()` 调用）。
