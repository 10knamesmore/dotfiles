---
name: rust-tdd
description: Use when writing or adding tests to any Rust crate, or implementing a Rust feature/bugfix test-first — covers choosing between assert_eq!, insta snapshots, proptest property tests, assert_cmd CLI tests, and process-level e2e, plus how to write assertions without unwrap and share fixtures across crates.
---

# Rust TDD：工具选型与落地

## Overview

**REQUIRED BACKGROUND：** TDD 纪律（Iron Law、RED-GREEN-REFACTOR 循环、"先看测试失败"）走
`superpowers:test-driven-development`，本 skill **不重复**那套纪律。

本 skill 只回答 Rust 落地问题：每一步**用哪个工具**、断言**怎么写不踩坑**、fixture **怎么跨 crate
共享**。RED-GREEN-REFACTOR 的"该写什么测试"先在脑子里定型，再用下面的选型矩阵选工具。

重型用法（各工具完整 API、e2e harness 模板、配置文件模板）见 `tool-reference.md`，**需要时再读**。

## 选型矩阵（先看这张表）

| 场景 | 工具 | 何时用它 |
|------|------|---------|
| 逻辑 / 算法 / 等价性 | `assert_eq!` / `assert!` | 期望值能写死在断言里 —— 这是默认选择，**别无谓上快照** |
| 渲染 / 多字段结构化输出 | **insta 快照** | 输出的整个"形状"重要、手写期望值太啰嗦（解析结果、终端渲染、序列化产物） |
| 纯函数不变量 | **proptest** | "对**任意**输入恒成立"：round-trip、解析器抗崩、数值范围约束 |
| CLI 黑盒 | **assert_cmd** + `predicates` | 起真二进制断言 exit code / stdout / stderr |
| 进程级 e2e | `env!("CARGO_BIN_EXE_<bin>")` + 真子进程 | 起真二进制、隔离环境、读日志、发信号验证生命周期 |
| doctest | `cargo test --doc` | nextest **不跑** doctest，必须单独兜 |

一句话决策：**单值/等价 → assert；形状 → insta；任意输入 → proptest；跨进程 → e2e。**
不确定时默认 `assert_eq!`，它最能表意。

## 运行器：cargo-nextest

默认运行器选 **cargo-nextest**（`cargo install cargo-nextest cargo-insta`）：并行快、可配 retries
吸收偶发 flaky、可用 test-groups 把抢资源的 e2e 串行隔离。**唯一坑**：nextest **不跑 doctest**，
doctest 必须单独 `cargo test --doc`。

推荐在 `.cargo/config.toml` 里固化别名（模板见 `tool-reference.md`）：

```bash
cargo t      # = nextest run --workspace   全仓测试
cargo td     # = test --workspace --doc    doctest（nextest 不覆盖，单独兜）
cargo snap   # = insta test --review       改了快照后人工审阅
```

## RED-GREEN-REFACTOR 在 Rust 的落地

- **RED**：先写测试。**编译不过也算红**（被测函数还不存在 → 编译错误是合法的"红"）。用 `cargo t`
  跑出红，确认它确实因为"行为缺失"而失败，而不是测试本身写错。
- **GREEN**：写最小实现让 `cargo t` 转绿。
- **REFACTOR**：保持绿。proptest 跑出的失败反例会落进 `proptest-regressions/<test>.txt`，
  **提交进 git** —— 它是永久回归用例，下次自动重跑防退化。

## 测试也是代码：断言与错误处理

测试不豁免工程标准。Rust 测试里常见的坏味道与正确写法：

```rust
// ❌ unwrap/expect 让失败信息变成无上下文的 panic
#[test]
fn round_trips() {
    let bytes = encode(&cfg);
    let got = decode(&bytes).unwrap();      // 失败只甩一句 called unwrap on Err
    assert_eq!(got, cfg);
}

// ✅ 测试函数返回 Result，用 ? 串接，断言用 assert_*
#[test]
fn round_trips() -> color_eyre::Result<()> {   // anyhow::Result / 自定义 Result 同理
    let bytes = encode(&cfg);
    let got = decode(&bytes)?;                  // 失败带完整 error 链冒泡
    assert_eq!(got, cfg);                       // 业务断言用 assert_eq!
    Ok(())
}
```

规矩：

- **测试里不用 `unwrap` / `expect` / 索引切片 `v[i]`**。用 `?` 冒泡、`assert_*` 断言、`.get(i)` 取值。
- 被测类型没实现 `PartialEq` 时，用 `format!("{:?}", x)` 比较 Debug 字符串（见 round-trip 样例）。
- 不要 `use` 裸 `Result`；签名直接写全 `-> color_eyre::Result<()>`，避免歧义。
- 对不透明字面量参数加注释：`spawn(/*force_null*/ true)`，名字与签名一致。

## fixture 共享

- **跨 crate 复用** → 抽一个独立的 `*-test` crate，集中放构造器（`fn config(id: &str) -> Config`）
  和函数式装饰器（`fn with_name(c: Config, name: &str) -> Config`）。各 crate 把它当 **dev-dependency**。
  （注：`*-test` dev-dep 你的 model crate、model crate 测试又 dev-dep `*-test`，Cargo **允许**这种
  仅经 dev-dependency 的环——dev-dep 不进正常构建图。）
- **crate 内私有** → `#[cfg(test)] mod test_support`，放本 crate 专属 fixture。
- **集成测试脱依赖** → 用 **null-object** 替身（no-op client、disabled fetcher）让测试不碰真网络 /
  socket / runtime，既快又确定。

## insta 快照纪律

1. **每张快照带描述**，`cargo insta review` 时逐张可辨。封一个强制描述的宏（模板见 `tool-reference.md`）：
   ```rust
   insta::with_settings!({ description => "状态栏:快捷键提示行" }, {
       insta::assert_snapshot!(rendered);
   });
   ```
2. `.snap` 文件**提交进 git**；新增/改动后 `cargo insta review` **人工逐张确认**。
3. **严禁盲接受**：不设 `INSTA_UPDATE=always`、不让自动化/agent 跑 `cargo insta accept`。
   CI 里设 `INSTA_UPDATE=no` 防漏审，未审的快照直接让 CI 红。生成的 `.snap.new` 留给人工 review。
4. 动态内容（时间戳、UUID、版本号）用 `filters` 归一化成占位符，否则快照永远在抖。

## 常见错误

| 坏味道 | 后果 | 正确做法 |
|--------|------|---------|
| 单值等价判断也上快照 | 期望值藏进 `.snap`，看测试看不出意图 | 用 `assert_eq!`，期望值写在断言里 |
| 测试里 `unwrap` / `expect` | 失败信息无上下文，难定位 | 返回 `Result` + `?` + `assert_*` |
| proptest 用在渲染 / IO | 不可复现、慢、断言难写 | 渲染/形状走 insta，IO 走 e2e |
| e2e 不隔离环境/共享 socket | flaky、互相干扰 | 每用例独立临时目录（PID+纳秒后缀），抢资源的用 test-group 串行 |
| 只跑 `cargo nextest`，漏 doctest | 文档示例腐烂没人发现 | 单独 `cargo test --doc`（`cargo td`） |
| 盲接受快照 / `INSTA_UPDATE=always` | 把 bug 一起接受成新基线 | `cargo insta review` 逐张人工确认 |

## 红旗——停下重来

- 测试里出现 `unwrap()` / `expect()` / `v[i]`
- "这个值用快照更省事"（单值等价该用 `assert_eq!`）
- "doctest 应该 nextest 已经跑了吧"（没有，单独跑）
- "先把快照 accept 了让 CI 过"（盲接受 = 把 bug 接受成基线）
- 先写实现再补测试（违反 `superpowers:test-driven-development`）
