# Rust 测试：compile-aware 验证与工具选型

## Overview

把 test target 的编译视为编译验证。不要为了制造 RED 而运行一个已知无法编译的中间状态，也不要在测试前机械地再跑一次 `cargo check`, 。

`编译10分钟证明了某个业务函数还不存在` 这种死板RED 在rust中， 不管是编译时间还是磁盘占用都是**无法接受**, 禁止用tmp目录存放target

先确认 compiler 已经证明了什么，再只测试 compiler 无法证明的行为。一次编辑完成最小实现和必要测试，然后选择覆盖目标行为的最窄命令，尽量只触发一次编译。

重型用法（各工具完整 API、e2e harness 模板、配置文件模板）见 `references/testing/tool-reference.md`，需要时再读。

## compiler 与测试的边界

- 类型正确性、ownership、lifetime、trait bound、enum exhaustiveness 等 compiler 已覆盖的性质，不写重复测试。
- 业务规则、算法结果、边界值、状态转换、解析与 serialization 语义、错误路径、并发和 process lifecycle 等 runtime behavior，用测试验证。
- regression 已有可复现输入时保留最小 regression test；不要为了形式上的 test-first，先写一个引用不存在 API 的测试并运行它。

## 选型矩阵（先看这张表）

| 场景 | 工具 | 何时用它 |
|------|------|---------|
| 逻辑 / 算法 / 等价性 | `assert_eq!` / `assert!` | 期望值能写死在断言里——这是默认选择，别无谓上快照 |
| 渲染 / 多字段结构化输出 | **insta 快照** | 输出的整个形状重要、手写期望值太啰嗦（解析结果、终端渲染、序列化产物） |
| 纯函数不变量 | **proptest** | 对任意输入恒成立：round-trip、解析器抗崩、数值范围约束 |
| CLI 黑盒 | **assert_cmd** + `predicates` | 起真二进制断言 exit code / stdout / stderr |
| 进程级 e2e | `env!("CARGO_BIN_EXE_<bin>")` + 真子进程 | 起真二进制、隔离环境、读日志、发信号验证生命周期 |
| doctest | `cargo test --doc` | nextest 不跑 doctest，必须单独兜 |

一句话决策：单值/等价 → assert；形状 → insta；任意输入 → proptest；跨进程 → e2e。
不确定时默认 `assert_eq!`，它最能表意。

## 运行器与最少编译

项目已配置 **cargo-nextest** 时优先使用：它并行快、可配 retries 吸收偶发 flaky、可用
test-groups 把抢资源的 e2e 串行隔离。nextest 不跑 doctest，相关 doctest 必须单独
`cargo test --doc`。

需要固化项目别名时参考 `references/testing/tool-reference.md`：

```bash
cargo t      # = nextest run；调用时显式传 -p / test filter
cargo td     # = test --doc；doctest（nextest 不覆盖，按相关 package 运行）
cargo snap   # = insta test --review       改了快照后人工审阅
```

命令按能提供的证据选择，不机械串行执行：

- 改动包含相关测试时，直接运行最窄的 test target 或 test filter；测试命令已编译 library 和 test target，不先跑 `cargo check`。
- 改动完全由 compiler 约束且没有必要的 behavior test 时，运行最窄的 `cargo check -p <crate>`。
- doctest、clippy、全 workspace 测试只在改动触及对应风险时追加；它们提供不同证据，但不作为每次修改后的固定流水线。
- 命令失败后根据诊断修改，再运行同一最窄命令。失败是诊断结果，不是必须刻意制造的流程阶段。
- proptest 跑出的失败反例会落进 `proptest-regressions/<test>.txt`，提交进 git 作为永久 regression case。

## 测试也是代码：断言与错误处理

测试不豁免工程标准。Rust 测试里常见的坏味道与正确写法：

```rust
// ❌ unwrap/expect 让失败信息变成无上下文的 panic
#[test]
fn round_trips() {
    let bytes = encode(&cfg);
    let got = decode(&bytes).unwrap();
    assert_eq!(got, cfg);
}

// ✅ 测试函数返回 Result，用 ? 串接，断言用 assert_*
#[test]
fn round_trips() -> color_eyre::Result<()> {
    let bytes = encode(&cfg);
    let got = decode(&bytes)?;
    assert_eq!(got, cfg);
    Ok(())
}
```

规矩：

- 测试里不用 `unwrap` / `expect` / 索引切片 `v[i]`。用 `?` 冒泡、`assert_*` 断言、`.get(i)` 取值。
- 被测类型没实现 `PartialEq` 时，用 `format!("{:?}", x)` 比较 Debug 字符串（见 round-trip 样例）。
- 不要 `use` 裸 `Result`；签名直接写全 `-> color_eyre::Result<()>`，避免歧义。
- 遵守入口 `SKILL.md` 的通用约定，不要给不透明字面量参数添加实参注释。

## fixture 共享

- **跨 crate 复用** → 抽一个独立的 `*-test` crate，集中放构造器（`fn config(id: &str) -> Config`）
  和函数式装饰器（`fn with_name(c: Config, name: &str) -> Config`）。各 crate 把它当 **dev-dependency**。
  `*-test` dev-dep model crate、model crate 测试又 dev-dep `*-test` 时，Cargo 允许这种仅经
  dev-dependency 的环，因为 dev-dependency 不进入正常构建图。
- **crate 内私有** → `#[cfg(test)] mod test_support`，放本 crate 专属 fixture。
- **集成测试脱依赖** → 用 **null-object** 替身（no-op client、disabled fetcher）让测试不碰真网络 /
  socket / runtime，既快又确定。

## insta 快照纪律

1. **每张快照带描述**，`cargo insta review` 时逐张可辨。封一个强制描述的宏（模板见 `references/testing/tool-reference.md`）：
   ```rust
   insta::with_settings!({ description => "状态栏:快捷键提示行" }, {
       insta::assert_snapshot!(rendered);
   });
   ```
2. `.snap` 文件提交进 git；新增/改动后 `cargo insta review` 人工逐张确认。
3. 严禁盲接受：不设 `INSTA_UPDATE=always`、不让自动化/agent 跑 `cargo insta accept`。
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
| 先 `cargo check` 再跑同一 target 的测试 | 重复编译，反馈变慢 | 直接跑最窄测试命令，把 test compilation 作为编译验证 |

## 红旗——停下重来

- 测试里出现 `unwrap()` / `expect()` / `v[i]`
- 单值等价判断用快照
- 认为 nextest 会运行 doctest
- 为了让 CI 通过而直接接受快照
- 为制造 RED 而运行引用不存在 API 的测试
- compiler 已覆盖的性质又写一层无行为价值的测试
- 局部改动无依据地跑全 workspace，或机械串行运行 `check`、`build`、`clippy`、test
