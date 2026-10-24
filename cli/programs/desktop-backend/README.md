# desktop-backend

Linux Quickshell 配置使用的 Rust 后端。`dots install` 构建并安装到 `~/.local/bin/desktop-backend`，QML 与脚本只引用安装后的命令。

| 子命令 | 职责 |
| --- | --- |
| `capture` | 启动并等待捕获服务；供 Quickshell 的 `Process` 跟踪 |
| `capture-picker [--allow-token]` | 转交 XDPH 来源选择请求，只输出用户明确确认的 selection |
| `network-status` | 订阅 IPv4 路由和 NetworkManager 变化，状态改变时输出 JSON 行 |
| `resource-monitor cpu\|memory\|network` | 为打开的资源面板每 100 ms 输出指标 |
| `system-info` | 查询软件版本和安装包数量，输出一次 JSON |

## 模块与生命周期

- `capture/`：桌面元数据、选区 BMP 转 PNG、剪贴板、Portal 来源选择和自有 GSR 录制。屏幕像素由 QML 的 `ScreencopyView` 采集；Rust 不重复采集屏幕。
- `capture-server` 是 `capture` 启动的内部子命令。Quickshell 重载时只结束外层进程，服务收到 stdin EOF 后拒绝待确认请求、停止并保存自有录制、清理临时目录，最后释放会话锁。
- `network_routes.rs`：通过 `rtnetlink` 查询网卡、IPv4 地址与主路由表，并订阅其变化。网络状态与资源面板共用这套接口；默认网卡取主路由表中 metric 最低的 IPv4 默认路由。
- `network_status/`：通过 zbus 读取 NetworkManager，由 D-Bus 与路由事件驱动，不运行轮询子进程。
- `resource_monitor/`：通过 `procfs` 获取 CPU tick、进程、内存、PSI 和网络计数；流量每 100 ms 采样，网卡地址与路由信息只在事件通知后更新。进程排行每 500 ms 刷新，CPU 频率和温度每秒从 `/sys` 刷新。CPU 进程占用以整机 100% 为分母。

stdout 仅用于 JSON 或 XDPH selection，诊断写 stderr。截图导出仍使用 `wl-copy`、`xdg-user-dir`；录屏仍使用 `gpu-screen-recorder`、Portal 和用于识别调用者的 `busctl`。详细操作见 `tree/home.linux/.config/quickshell/capture/README.md`。

AI usage 独立使用 `tree/home.linux/.config/quickshell/ai-usage/collect.py`，不属于此程序。

## 本地构建

```bash
cargo check --manifest-path cli/programs/desktop-backend/Cargo.toml
cargo clippy --manifest-path cli/programs/desktop-backend/Cargo.toml -- -D warnings
cargo build --release --manifest-path cli/programs/desktop-backend/Cargo.toml
```
