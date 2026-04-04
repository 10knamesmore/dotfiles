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

`install.py` 只处理符号链接和模板渲染，以下配置需要手动安装（通常需要 root 权限）。

---

## 1. 内置键盘自动禁用（外接键盘插入时）

### 原理

- udev 规则监听外接键盘的 USB 热插拔事件，触发脚本写 sysfs `inhibited` 接口
- systemd 服务在开机时做一次状态同步（处理开机时已连接外接键盘的情况）

### 涉及文件

| 文件 | 说明 |
|------|------|
| `linux/udev/99-keyboard-inhibit.rules` | udev 规则，监听已知外接键盘 |
| `linux/scripts/hypr/keyboard_manager.sh` | 实际执行 enable/disable/sync 的脚本 |
| `linux/systemd/keyboard-inhibit-sync.service` | 开机同步服务 |

### 安装步骤

```bash
# 1. udev 规则
sudo cp ~/dotfiles/linux/udev/99-keyboard-inhibit.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules

# 2. systemd 服务（处理开机已连接的情况）
sudo cp ~/dotfiles/linux/systemd/keyboard-inhibit-sync.service /etc/systemd/system/
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
sudo ~/dotfiles/linux/scripts/hypr/keyboard_manager.sh disable
sudo ~/dotfiles/linux/scripts/hypr/keyboard_manager.sh enable

# 查看当前 inhibit 状态
cat /sys/class/input/input2/inhibited   # AT Translated Set 2 keyboard
cat /sys/class/input/input3/inhibited   # ITE Device(8296) Keyboard

# 查看 udev 触发日志
journalctl -f | grep keyboard_manager
```
