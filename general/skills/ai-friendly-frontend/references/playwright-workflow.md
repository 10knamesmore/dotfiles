# Playwright 工作循环：用 `playwright-cli` 维护与沉淀

本项目用 **[microsoft/playwright-cli](https://github.com/microsoft/playwright-cli)**（包名 `@playwright/cli`，命令 `playwright-cli`）作为 agent 操作浏览器的入口。它是专门为 coding agent 设计的 token-efficient CLI，与 `npx playwright test`（测试 runner）和 Playwright MCP 都不同：

| 工具 | 用途 | 何时用 |
|------|------|------|
| `playwright-cli` | agent 实时操作浏览器、抓 a11y snapshot、生成 locator | **写完代码立刻验证** / 调试 / 修复挂的测试 |
| `npx playwright test` | 跑 `.spec.ts`，CI 与本地回归 | **沉淀阶段** —— 把 `playwright-cli` 验证通过的交互写成 spec |
| Playwright MCP | 持续状态的 agentic loop、自愈测试 | 本 skill 不用 |

工作循环的核心信号：**`playwright-cli snapshot` 输出的 accessibility 树就是 AI agent 看到的页面**。如果你的元素在 snapshot 里以正确的 `role` + `name` 出现，就是 AI-friendly；如果是无名 `generic`、`text`，就回去改前端。

## 前置

```bash
# 一次性
npm install -g @playwright/cli@latest
playwright-cli --version

# 可选：安装官方 skills 到 Claude Code / Copilot 本地
playwright-cli install --skills
```

> 官方 skills 给的是「如何用 playwright-cli」；本 skill 在它之上加 AntD / 自研组件库 / 前端代码契约。两者互补，可以同时装。

## 核心循环：snapshot → 操作 → generate-locator → 沉淀 spec

写完一段 UI，**立刻**走这个流程：

```bash
# 1. 打开页面（headed 方便观察，headless 更快）
playwright-cli open http://localhost:5173/login --headed

# 2. 抓 accessibility 快照 —— 这一步是关键诊断
playwright-cli snapshot
```

`snapshot` 输出形如：

```
- generic [ref=e1]:
  - heading "登录" [level=1] [ref=e2]
  - textbox "邮箱" [ref=e10]
  - textbox "密码" [ref=e15]
  - button "登录" [ref=e20]
  - link "忘记密码" [ref=e25]
```

**读这份输出，判断前端 AI-friendly 程度**：

| snapshot 输出 | 含义 | 行动 |
|---------------|------|------|
| `button "登录"` | 完美：原生 button + 可见文本 | 用 `getByRole('button', { name: '登录' })` |
| `button` 没有 name | 图标按钮缺 `aria-label` | **回去补 `aria-label`** |
| `generic "X"` 或 `text "X"` 用作交互 | 用了 `<div onClick>` | **回去改成 `<button>`** |
| 元素根本没出现 | DOM 没渲染 / `aria-hidden` 错配 | 调查为什么 |
| `textbox` 没有 name | 表单字段缺 label 绑定 | **补 `<Form.Item label>` 或 `aria-label`** |

snapshot 通不过 = 前端不 AI-friendly，不是 agent 不行。回去修代码再 snapshot。

## 用 ref 操作

snapshot 给的 `ref=e10` 是临时 id，可以直接用：

```bash
playwright-cli fill e10 "user@example.com"
playwright-cli fill e15 "secret"
playwright-cli click e20

# 操作后状态变了，重新抓 snapshot
playwright-cli snapshot
```

每次操作改变了 DOM，**ref 可能失效**，要重抓 snapshot。

## generate-locator：把 ref 翻译成稳定 locator

ref 是临时的，spec 文件里不能用 ref。用 `generate-locator` 把它翻译成 Playwright 的稳定 locator：

```bash
playwright-cli generate-locator e20
# 输出：page.getByRole('button', { name: '登录' })

playwright-cli generate-locator e10
# 输出：page.getByLabel('邮箱')
```

**这是 AI-friendly 程度的金标准**：

- 输出 `getByRole` / `getByLabel` → 完美，直接用。
- 输出 `getByText` → 还可以，但 i18n 时脆。
- 输出 `getByTestId` → 兜底成功（你前面加了 testid）。
- 输出 `locator('div').filter(...)` / 用 CSS class → **失败**，前端缺锚点，回去补。

## 完整示例：登录流程从 0 到 .spec.ts

```bash
# 1) 打开 + 看初始 snapshot
playwright-cli open http://localhost:5173/login --headed
playwright-cli snapshot

# 假设输出：
# - textbox "邮箱" [ref=e10]
# - textbox "密码" [ref=e15]
# - button "登录" [ref=e20]

# 2) 操作走一遍
playwright-cli fill e10 "user@example.com"
playwright-cli fill e15 "correct"
playwright-cli click e20

# 3) 看结果 snapshot —— 异步态是否投射到 DOM？
playwright-cli snapshot
# 期望看到：button "登录" [busy] 或 data-state="loading"

# 4) 等异步完成，再 snapshot
playwright-cli snapshot
# 期望：跳到 dashboard 或出现 alert

# 5) 把每个交互翻译成 locator
playwright-cli generate-locator e10  # → page.getByLabel('邮箱')
playwright-cli generate-locator e15  # → page.getByLabel('密码')
playwright-cli generate-locator e20  # → page.getByRole('button', { name: '登录' })

# 6) 拍个截图存档
playwright-cli screenshot --filename=login-success.png
```

把 5 的输出组装成 `tests/login.spec.ts`：

```ts
import { test, expect } from '@playwright/test';

test('成功登录跳转到 dashboard', async ({ page }) => {
  await page.goto('/login');
  await page.getByLabel('邮箱').fill('user@example.com');
  await page.getByLabel('密码').fill('correct');
  const submit = page.getByRole('button', { name: '登录' });
  await submit.click();

  // 状态投射可观察
  await expect(submit).toHaveAttribute('aria-busy', 'true');

  // 跳转
  await expect(page).toHaveURL(/\/dashboard$/);
});
```

跑：

```bash
npx playwright test tests/login.spec.ts
```

通过即沉淀完成。

## 常用 `playwright-cli` 命令速查

### 浏览与操作

```bash
playwright-cli open [url]                    # 打开，可省 url 复用已开页面
playwright-cli goto <url>                    # 在当前页导航
playwright-cli click <ref>                   # 按 ref 点击
playwright-cli dblclick <ref>                # 双击
playwright-cli fill <ref> <text>             # 填入文本
playwright-cli fill <ref> <text> --submit    # 填后回车
playwright-cli type <text>                   # 直接键盘输入到当前焦点
playwright-cli press <key>                   # 按键：Enter / Tab / Escape / ArrowDown
playwright-cli hover <ref>                   # hover
playwright-cli select <ref> <value>          # 下拉选项（原生 <select>）
playwright-cli check <ref>                   # 勾选 checkbox/radio
playwright-cli upload <file>                 # 文件上传
```

### 观察

```bash
playwright-cli snapshot                      # 抓 a11y 快照（最常用）
playwright-cli snapshot <ref>                # 只抓某个元素子树
playwright-cli snapshot --depth=3            # 限制深度避免噪音
playwright-cli screenshot                    # 截当前页
playwright-cli screenshot <ref>              # 截某元素
playwright-cli screenshot --filename=foo.png
playwright-cli console                       # 看 console 消息
playwright-cli console error                 # 只看 error
playwright-cli requests                      # 看所有网络请求
playwright-cli request <index>               # 看某请求详情
playwright-cli eval "() => document.title"   # 执行 JS
```

### Locator 生成

```bash
playwright-cli generate-locator <ref>        # ref → 稳定 locator（核心命令）
playwright-cli highlight <ref>               # 在页面上高亮，方便定位
playwright-cli highlight --hide              # 清除高亮
```

### 会话管理

`playwright-cli` 默认把 cookie / storage 保留在内存里，进程退出就丢。

```bash
playwright-cli open url                              # 默认 session
playwright-cli -s=todo open url --persistent         # 命名 + 持久化到磁盘
playwright-cli list                                  # 看所有 session
playwright-cli close-all                             # 关掉所有 session
playwright-cli kill-all                              # 强杀

# 把整轮 agent 操作绑到一个 session
PLAYWRIGHT_CLI_SESSION=login-flow claude .
```

### 网络 mock

```bash
playwright-cli route '**/api/users' --status=500     # mock 失败
playwright-cli route-list                            # 列出当前规则
playwright-cli unroute '**/api/users'                # 解除
```

测试错误态时极有用：让 API 返回 500，然后 snapshot 看错误信息是否真的有 `role="alert"`。

### 可视化面板

```bash
playwright-cli show                          # 打开 dashboard 看所有 session 实况
playwright-cli show --annotate               # UI review / 设计反馈模式
```

dashboard 里可以看每个 session 的实时画面，点进去能直接接管鼠键操作——agent 在跑、人想插手时用。

### 状态持久化

```bash
playwright-cli state-save state.json         # 保存 cookie + localStorage
playwright-cli state-load state.json         # 跨 session 复用登录态
```

写测试时用一次保存登录态，后续测试直接 `state-load` 跳过登录。

## 用 `playwright-cli` 修复挂掉的测试

收到任务"`login.spec.ts` 挂了"：

```bash
# 1) 先用 spec runner 复现失败
npx playwright test tests/login.spec.ts --headed

# 2) 失败后用 CLI 进入同一页面手动复现
playwright-cli open http://localhost:5173/login --headed
playwright-cli snapshot

# 3) 比对 snapshot 与 spec 期望
#    spec 用了 page.getByLabel('邮箱')，但 snapshot 里现在是 "电子邮件"？
#    → 文案改了，spec 要更新（或 i18n 切了，要正则匹配）
#    snapshot 里 button 没了 name？
#    → 前端 bug，回去补 aria-label

# 4) 用 generate-locator 验证新的稳定 locator
playwright-cli generate-locator <ref>

# 5) 改 spec 或前端，再跑 npx playwright test
```

**判断改哪边**：

- snapshot 显示元素**语义降级**了（button → generic）→ 前端 bug，改前端。
- snapshot 显示文案/标签变了但语义还在 → spec 该更新。
- snapshot 显示新加了一步交互 → spec 加一步。
- snapshot 完全找不到目标元素 → 调查路由 / 权限 / 数据状态。

## 视觉回归

`playwright-cli` 的 `screenshot` 可以快照，但**视觉回归用 `@playwright/test` 的 `toHaveScreenshot`**：

```ts
test('登录页视觉', async ({ page }) => {
  await page.goto('/login');
  await page.waitForFunction(() => document.fonts.ready.then(() => true));
  await expect(page).toHaveScreenshot('login.png');
});
```

测试环境务必：

- 关 AntD 动画：`ConfigProvider theme={{ token: { motion: false } }}`。
- 等字体：`document.fonts.ready`。
- 屏蔽动态区域：`{ mask: [page.getByTestId('current-time')] }`。

更新基线只更新涉及的，不要 `--update-snapshots` 全量。

## 禁止事项

- ❌ 在 `.spec.ts` 里写 `playwright-cli` 命令。CLI 是过程性工具，spec 用 `@playwright/test` API。
- ❌ 把 ref（`e10` 这种）写进 spec —— ref 一次性，会失效。
- ❌ 用 `page.waitForTimeout(N)` 凑时间——前端没投射状态就回去补 `aria-busy` / `data-state`，让 `toHaveAttribute` 自动等。
- ❌ `--update-snapshots` 一把梭（掩盖回归）。
- ❌ 给每个元素都加 `data-testid`：snapshot 输出已经是 `button "登录"`，再加 testid 是噪音。
- ❌ 把临时验证脚本（`/tmp/probe.mjs`）当测试代码提交。

## 一句话总结

**`playwright-cli snapshot` 的输出就是 AI agent 看页面的样子。让它干净（正确的 role + name），前端就 AI-friendly 了；`generate-locator` 输出 `getByRole` / `getByLabel`，spec 就能稳定。两个信号都达标，把交互翻译成 `.spec.ts` 就是顺手的事。**
