# desktop-backend

Linux Quickshell 配置使用的 Rust 后端。`dots install` 构建并安装到 `~/.local/bin/desktop-backend`，QML 与脚本只引用安装后的命令。

| 子命令 | 职责 |
| --- | --- |
| `capture` | 启动并等待捕获服务；供 Quickshell 的 `Process` 跟踪 |
| `capture-picker [--allow-token]` | 转交 XDPH 来源选择请求，只输出用户明确确认的 selection |
| `bluetooth` | 在同一 D-Bus 会话管理蓝牙设备，stdin/stdout 使用交互式 JSON 行协议 |
| `network-status` | 订阅 IPv4 路由和 NetworkManager 变化，状态改变时输出 JSON 行 |
| `resource-monitor cpu\|memory\|network` | 为打开的资源面板每 100 ms 输出指标 |
| `system-info` | 查询软件版本和安装包数量，输出一次 JSON |

## 模块与生命周期

- `capture/`：桌面元数据、选区 BMP 转 PNG、剪贴板、Portal 来源选择和自有 GSR 录制。屏幕像素由 QML 的 `ScreencopyView` 采集；Rust 不重复采集屏幕。
- `capture-server` 是 `capture` 启动的内部子命令。Quickshell 重载时只结束外层进程，服务收到 stdin EOF 后拒绝待确认请求、停止并保存自有录制、清理临时目录，最后释放会话锁。
- `network_routes.rs`：通过 `rtnetlink` 查询网卡、IPv4 地址与主路由表，并订阅其变化。网络状态与资源面板共用这套接口；默认网卡取主路由表中 metric 最低的 IPv4 默认路由。
- `network_status/`：通过 zbus 读取 NetworkManager，由 D-Bus 与路由事件驱动，不运行轮询子进程。
- `bluetooth/`：读取 BlueZ ObjectManager，订阅对象与属性变化；注册仅供本会话使用的 `KeyboardDisplay` Agent，不申请 default agent。操作并发执行，配对等待确认时仍可接收 stdin。
- `resource_monitor/`：通过 `procfs` 获取 CPU tick、进程、内存、PSI 和网络计数；流量每 100 ms 采样，网卡地址与路由信息只在事件通知后更新。进程排行每 500 ms 刷新，CPU 频率和温度每秒从 `/sys` 刷新。CPU 进程占用以整机 100% 为分母。

stdout 仅用于 JSON 或 XDPH selection，诊断写 stderr。截图导出仍使用 `wl-copy`、`xdg-user-dir`；录屏仍使用 `gpu-screen-recorder`、Portal 和用于识别调用者的 `busctl`。详细操作见 `tree/home.linux/.config/quickshell/capture/README.md`。

AI usage 独立使用 `tree/home.linux/.config/quickshell/ai-usage/collect.py`，不属于此程序。

## Bluetooth 协议

启动后先输出 `{type:"state",adapters:[…],devices:[…]}`，Agent 注册成功后输出 `{type:"ready"}`；后续仅在 D-Bus 事件引起状态变化时更新 state，无定时轮询。名称取 BlueZ `Alias`；设备包含 paired、bonded、trusted、blocked、uuids、servicesResolved，rssi 与 battery 未知时为 null，battery 单位为百分比。

stdin 每行一个 JSON 对象：

- `{id,action,device}`：`connect`、`disconnect`、`cancel`、`forget`。
- `{id,action:"trust",device,value:bool}`。
- `{id,action:"power"|"scan",adapter,value:bool}`。

device 与 adapter 均为 D-Bus object path。界面串行提交连接、断开、忘记、电源与扫描操作；`trust` 静默更新属性，不占用页面的操作中状态，`cancel` 与提示回应可在操作未完成时提交。`connect` 在尚未 paired 或 bonded 时先 Pair，再 Connect，不改变 Trusted。输出 `{type:"operation",id,device,action,stage,error?}`；stage 为 pairing、connecting、done 或 error，适配器操作的 device 为 `""`，error 保留 BlueZ 错误名称与原因。

Agent 输出 `{type:"prompt",requestId,device,kind,passkey?,entered?,uuid?}`；kind 为 confirmation、pin、passkey、displayPin、displayPasskey、authorization 或 service。数值 passkey 为六位补零字符串；entered 是远端已输入位数。回应使用 `{action:"respond",requestId,accept:bool,value?:string}`。PIN 为 1–16 字符且不含 NUL；passkey 为 1–6 位十进制数字（0–999999）。无效回应写 stderr 并保留提示，允许更正。displayPin 与 displayPasskey 只展示，不回应；同一 requestId 的 displayPasskey 更新 entered。回应、BlueZ 取消或配对结束时输出 `{type:"promptClosed",requestId}`。

stdin EOF、SIGINT 或 SIGTERM 会拒绝待确认请求、取消本会话尚未完成的配对或连接、释放本会话 discovery 并注销 Agent，不关闭适配器。其他客户端的 discovery 会话不受影响。

## 本地构建

```bash
cargo check --manifest-path cli/programs/desktop-backend/Cargo.toml
cargo clippy --manifest-path cli/programs/desktop-backend/Cargo.toml -- -D warnings
cargo build --release --manifest-path cli/programs/desktop-backend/Cargo.toml
```
