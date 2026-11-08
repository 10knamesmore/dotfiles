# Pi Design

浏览器设计工作区与当前 Pi TUI 会话共用一个作品。手动编辑、调参和评论
自动保存；TUI 发消息时附带设计上下文。作品是普通 React/Vite 项目，
源码和设计 JSON 都支持直接文件编辑。

## 使用

`/design` 打开全局项目选择器。选择已有项目恢复工作区；选择 **New**
后只有一个空的自由输入框。提交的原文进入当前会话，模型按 design skill
创建项目并调用 Python SDK 注册、打开。

```python
import pi_design

project = pi_design.current()
state = project.read_state()
```

项目清单在 `~/.pi/agent/design/projects.json`。每个 Pi 会话的本地服务
负责自己的活动项目和预览进程；退出或 reload 时关闭。项目源码、参数和评论
保留在项目目录，下一次选择项目后继续。

编辑器默认暗色，右上角切换亮色；选择保存在
`~/.pi/agent/design/preferences.json`，不改变作品配色。右下角滑块控制
画布缩放，悬停滑块滚动时向上放大、向下缩小。
缩放与滚轮平移带短过渡动画；鼠标直接拖动即时跟手。底部手形工具启用时，
画布上的滚轮控制缩放。两侧的图层展开、面板切换和选中反馈使用轻量过渡。
左侧 Layers 下的 History 可切换当前打开期间的设计数据快照，包含参数、
文字、样式、画板和评论，不包含 React 源码；重启服务后重新记录。
参数滑块拖动中实时保存，松手后只生成一条历史；拖回起始值不新增快照。

## 结构

| 目录或文件 | 职责 |
| --- | --- |
| `index.ts`、`selector.ts` | `/design`、会话关联、TUI 输入上下文 |
| `service.ts` | 本地服务进程的启动和关闭 |
| `native/` | Rust 状态服务与 PyO3 Python SDK |
| `web/` | React 画布、属性面板、参数与评论 |
| `react/` | 作品中的可编辑 React 组件和浏览器通信 |
| `skills/design/` | 模型的创建、修改和验证流程 |
| [protocol.md](protocol.md) | 项目文件、HTTP 与浏览器消息格式 |

普通第三方依赖安装在作品自己的 `package.json` 中。编辑器全局安装，
作品只依赖小型 `@pi-design/react` 运行时。

## 构建与安装

从仓库根目录执行：

```sh
pnpm --dir pi install
pnpm --dir pi run build:design
uv sync --project pi/src/extensions/python --locked
uv run --project pi/src/extensions/python --no-sync python pi/src/extensions/python/environment_manifest.py
pnpm --dir pi run typecheck:design
```

正式分发使用 `./dots.sh sync`：安装 Python 原生包、构建 Web 与运行时、
写环境清单并链接扩展。之后在 Pi 中 `/reload`。

运行时包的构建产物在 `react/dist/`，Web 产物在 `web/dist/`，均不入库。
服务日志在 `~/.pi/agent/design/logs/<session-id>.log`。
