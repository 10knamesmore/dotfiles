# 需要手动安装的配置项

<!--toc:start-->
- [需要手动安装的配置项](#需要手动安装的配置项)
  - [1. 内置键盘自动禁用（外接键盘插入时）](#1-内置键盘自动禁用外接键盘插入时)
    - [原理](#原理)
    - [涉及文件](#涉及文件)
    - [安装步骤](#安装步骤)
    - [已知外接键盘（keyboard_manager.sh 中维护）](#已知外接键盘keyboardmanagersh-中维护)
    - [调试](#调试)
<!--toc:end-->

`dots sync` 只管理 `$HOME` 侧的符号链接，以下 root 级配置需要手动安装。

---

## 1. 内置键盘自动禁用（外接键盘插入时）

### 原理

- udev 规则监听外接键盘的 USB 热插拔事件，触发脚本写 sysfs `inhibited` 接口
- systemd 服务在开机时做一次状态同步（处理开机时已连接外接键盘的情况）

### 涉及文件

root 级文件源在仓库 `system/`（dots 不链接它们，需手动 `cp` 到 `/etc`）：

| 文件 | 说明 |
|------|------|
| `system/udev/99-keyboard-inhibit.rules` | udev 规则，监听已知外接键盘 |
| `system/systemd/keyboard-inhibit-sync.service` | 开机同步服务 |
| `scripts/linux/hypr/keyboard_manager.sh` | 实际执行 enable/disable/sync 的脚本 |

### 安装步骤

> 更新 `system/` 下的源文件后，需重跑安装步骤（`/etc` 是 cp 的副本，不会自动跟随仓库）。
> 核对是否同步：`diff /etc/udev/rules.d/99-keyboard-inhibit.rules system/udev/99-keyboard-inhibit.rules`

```bash
cd ~/dotfiles
# 1. udev 规则
sudo cp system/udev/99-keyboard-inhibit.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules

# 2. systemd 服务（处理开机已连接的情况）
sudo cp system/systemd/keyboard-inhibit-sync.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now keyboard-inhibit-sync.service
```

### 已知外接键盘（keyboard_manager.sh 中维护）

- `320f:5088` — Telink Wireless Gaming Keyboard
- `1313:4122` — Redox Customized

新增外接键盘时，需同时更新 `keyboard_manager.sh` 的 `EXTERNAL_KEYBOARDS` 列表和 `99-keyboard-inhibit.rules`，然后重新执行安装步骤。

### 调试

```bash
# 手动测试禁用/启用
sudo ~/dotfiles/scripts/linux/hypr/keyboard_manager.sh disable
sudo ~/dotfiles/scripts/linux/hypr/keyboard_manager.sh enable

# 查看当前 inhibit 状态（input 编号随设备枚举漂移，先按名字找再 cat）
grep -iE 'AT Trans|ITE.*Keyboard' /sys/class/input/input*/name
cat /sys/class/input/input<N>/inhibited

# 查看 udev 触发日志
journalctl -f | grep keyboard_manager
```
