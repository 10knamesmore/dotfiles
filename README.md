# Dotfiles

Personal dotfiles managed with symbolic links.

## 结构

```plaintext
.
├── MacOS/
├── Linux/
├── General/
├── static/
├── generated/
└── install.sh
```

MacOS, Linux, General里面的结构以~/ 为根目录整理

static 存放不创建软链接的配置

有些配置由install.sh 动态生成, 比如渲染后的.template文件会被放到generated

最终符号链接会指向 generated

## 输出命令

- dot : cd 到 dotfile 项目目录

## 依赖

TODO

## 安装

```bash
./install.sh
```

脚本针对zsh 编写, 使用bash会有兼容问题!

这个脚本会:

1. 检测你的操作系统

2. 将现有dotfiles备份到 `backup/`

3. 创建符号链接

## 备份恢复

TODO
