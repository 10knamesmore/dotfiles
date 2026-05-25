# Rust 测试工具参考

主流程与选型见 `SKILL.md`。本文件是各工具可直接抄用的模板，去项目化、换成通用占位类型
（`Config` / `Request` / `myapp` 等），需要哪段抄哪段。

## cargo 别名（`.cargo/config.toml`）

```toml
[alias]
# 需 `cargo install cargo-nextest cargo-insta`
t    = "nextest run --workspace"      # 全仓测试
td   = "test --workspace --doc"       # doctest（nextest 不覆盖，单独兜）
snap = "insta test --test-runner nextest --review"  # 改了快照后人工审阅；绝不 accept-all
```

## nextest 配置（`.config/nextest.toml`）

```toml
[profile.default]
# 偶发 flaky 自动重试一次再判失败；卡死的测试 60s 超时、最多重启 2 次。
retries = 1
slow-timeout = { period = "60s", terminate-after = 2 }

[test-groups]
# 抢全局资源（unix socket / 端口 / 真子进程）的 e2e 限单线程串行，杜绝并发互相干扰。
e2e = { max-threads = 1 }

[[profile.default.overrides]]
filter = "binary(daemon_lifecycle)"   # 按测试二进制名挂到串行组
test-group = "e2e"
```

按测试名过滤跑：`cargo nextest run -E 'test(round_trips)'`。

## insta 快照

`Cargo.toml`：`insta = { version = "1", features = ["filters"] }`（dev-dependency）。

```rust
// Display 快照：终端渲染、序列化文本
insta::assert_snapshot!(rendered);

// Debug 快照：结构体、解析结果
insta::assert_debug_snapshot!(parsed);

// 带描述 + 归一化 + map 排序
insta::with_settings!({
    description => "解析结果:含翻译行的歌词",       // review 时逐张可辨
    sort_maps => true,                              // HashMap 顺序不确定时锁定
    filters => vec![(r"\d{4}-\d{2}-\d{2}", "[DATE]")],  // 动态内容归一化
}, {
    insta::assert_debug_snapshot!(parsed);
});
```

**强制描述的共享宏**（封在你的 `*-test` crate 里，宏体在调用方展开，故调用方 crate 需自带
`insta` dev-dependency）：

```rust
/// 带描述的 Display 快照断言。用法：`assert_snap!("描述", value);`
#[macro_export]
macro_rules! assert_snap {
    ($desc:expr, $value:expr $(,)?) => {{
        insta::with_settings!({ description => $desc }, {
            insta::assert_snapshot!($value);
        });
    }};
}
```

**审阅流程**：改动产生 `.snap.new` → `cargo insta review` 逐张 a(ccept)/r(eject) →
确认无误才 `.snap` 进 git。CI 设 `INSTA_UPDATE=no`，未审快照让 CI 红。**绝不 `INSTA_UPDATE=always`。**

## proptest

`Cargo.toml`：`proptest = "1"`（dev-dependency）。

```rust
use proptest::prelude::{Strategy, any, prop_oneof, proptest};
use proptest::collection::vec;
use proptest::test_runner::TestCaseError;

// 手写 Strategy：组合 prop_oneof! + prop_map（也可用 proptest-derive 的 #[derive(Arbitrary)]）
fn arb_request() -> impl Strategy<Value = Request> {
    prop_oneof![
        proptest::prelude::Just(Request::Pause),
        any::<u64>().prop_map(Request::Seek),
        (vec(any::<u8>(), 0..64)).prop_map(Request::Blob),
    ]
}

proptest! {
    // round-trip：任意输入编解码恒等
    #[test]
    fn request_roundtrip(req in arb_request()) {
        let bytes = encode(&req);
        // 被测 Error 实现 std::error::Error 即可 ?；否则 map_err 成 TestCaseError
        let back = decode(&bytes).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(back, req);   // 无 PartialEq 时比较 format!("{:?}", ...)
    }

    // 抗崩：任意字节喂 decode，要么 Ok 要么 Err，绝不 panic
    #[test]
    fn decode_never_panics(bytes in vec(any::<u8>(), 0..4096)) {
        let _ = decode(&bytes);   // 不 unwrap；跑到这没 panic 即过
    }
}
```

**失败反例落盘**：proptest 把最小反例写进 `proptest-regressions/<module>.txt`，**提交进 git**，
下次自动重跑作回归护栏。

## assert_cmd（CLI 黑盒 e2e）

`Cargo.toml`：`assert_cmd = "2"`、`predicates = "3"`（dev-dependency）。

```rust
use assert_cmd::Command;
use predicates::str::contains;

// 隔离环境的命令工厂（env 指向唯一临时目录，避免读到真配置）
fn myapp() -> color_eyre::Result<Command> {
    let root = std::env::temp_dir().join(format!(
        "myapp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let mut cmd = Command::cargo_bin("myapp")?;   // 定位 target/ 下的真二进制
    cmd.env("XDG_CONFIG_HOME", root.join("config"));
    Ok(cmd)
}

#[test]
fn help_exits_zero() -> color_eyre::Result<()> {
    myapp()?.arg("--help").assert().success();
    Ok(())
}

#[test]
fn missing_server_errors_with_hint() -> color_eyre::Result<()> {
    myapp()?.arg("status").assert()
        .failure()                       // 非零退出
        .stderr(contains("serve"));      // 错误信息给出修复提示
    Ok(())
}
```

## 进程级 e2e（真子进程 + 生命周期）

集成测试（`tests/` 下）里，`env!("CARGO_BIN_EXE_<bin>")` 拿到被测二进制路径。封一个 RAII
harness：spawn 真子进程、隔离临时目录、轮询就绪、发信号、Drop 清理。骨架：

```rust
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use color_eyre::eyre::{WrapErr, bail};

/// 一个隔离环境的 server 子进程；Drop 时 kill + 清临时目录。
struct Server { child: Child, root: PathBuf, socket: PathBuf }

impl Server {
    fn spawn(tag: &str) -> color_eyre::Result<Self> {
        let root = std::env::temp_dir().join(format!("myapp-e2e-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&root).wrap_err("create isolated dir")?;
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_myapp"));
        cmd.arg("serve")
            .env("XDG_RUNTIME_DIR", &root)
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        let child = cmd.spawn().wrap_err("spawn server")?;
        let socket = root.join("myapp.sock");
        Ok(Self { child, root, socket })
    }

    /// 轮询直到 socket 可连（就绪探测），超时报错。
    fn wait_ready(&self) -> color_eyre::Result<()> {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if std::os::unix::net::UnixStream::connect(&self.socket).is_ok() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        bail!("server 未在超时内就绪")
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
```

要点：

- **每个测试独立临时目录**（PID + 纳秒后缀），互不干扰、可并行。
- 抢全局资源的（socket / 端口）在 `.config/nextest.toml` 挂 test-group 串行。
- 发信号用 `nix` crate（`kill(Pid, Signal::SIGTERM)`），验证 graceful shutdown / socket 清理。
- 读日志、断言退出码、断言副作用，覆盖单元测试碰不到的进程时序。
