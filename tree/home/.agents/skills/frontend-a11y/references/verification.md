# 验证:证明 agent 能定位

声明"可访问/可定位"前,跑下面的检查并贴输出。本文件自包含,不依赖任何外部 skill/工具。

## 起步(一次性)

```bash
# 装 Playwright(含浏览器)。已有则跳过。
pnpm add -D @playwright/test @axe-core/playwright && pnpm exec playwright install chromium
```

页面跑起来后(dev server / Storybook / 静态 html 均可),下面任选:
- 快速一次性:`node check.mjs`(把片段存成 `check.mjs`,`import { chromium } from 'playwright'` 起页)。
- 纳入用例:写进 `*.spec.ts`,`pnpm exec playwright test`。
两者 API 相同,下文片段直接可用。

## 1. accessibility snapshot(最直接)

看每个可交互/有信息元素是否都带 role + name 出现在树里。

### Playwright — ARIA snapshot(YAML)
```js
const yaml = await page.locator('body').ariaSnapshot();
console.log(yaml);
// 期望能看到:  - button "Subscribe"   - textbox "邮箱"   - dialog "确认删除" …
// 出现 role 但无 name(如裸 button、generic)= 该元素 agent 定位不了,回去补 name
```
断言式:
```js
await expect(page.locator('form')).toMatchAriaSnapshot(`
  - textbox "邮箱"
  - button "Subscribe"
`);
```

### CDP — 全量 AX 树
```js
const client = await page.context().newCDPSession(page);
await client.send('Accessibility.enable');
const { nodes } = await client.send('Accessibility.getFullAXTree');
// 检查交互节点的 role.value 与 name.value 是否齐全、非空、不重名
```

## 2. locator 唯一性(strict mode)

Playwright 默认 strict:命中多个即抛错,正好用来查重名。

```js
await expect(page.getByRole('button', { name: 'Subscribe' })).toHaveCount(1);
await page.getByRole('textbox', { name: '邮箱' }).fill('a@b.com');
await page.getByRole('button', { name: 'Subscribe' }).click();
// 报 "resolved to 2 elements" = 有重名,给 name 加上下文消歧
```

优先级(越靠前越贴近用户/agent 真实感知):
`getByRole` > `getByLabel` / `getByPlaceholder` > `getByText` / `getByAltText` / `getByTitle` > `getByTestId`。
需要 `getByTestId` 才定位得到,说明语义/name 缺失,先补语义。

**i18n 例外**:当文案随语言/产品微调变动时,`getByTestId` 优先于 `getByText`(text 是巧合、testid 是契约)。给关键交互加稳定 `data-testid`(可用翻译 key),或 name 用正则匹配多语言 `getByRole('button',{name:/登录|sign in/i})`。

表单字段的重名同样要查(不只按钮):
```js
await expect(page.getByLabel('城市')).toHaveCount(1);  // 账单/收货两组都有"城市"会命中 2
```

**禁止作 locator**(出现即说明语义/name/testid 没做够,回去补,别在选择器侧硬抓):
`.css-1a2b3c`(CSS-in-JS 哈希)、`.ant-btn`(库前缀类)、`:nth-child(3)`(DOM 顺序)、`//div[2]/button[1]`(xpath 索引)、任何按视觉层级拼的 CSS 选择器。generate-locator 退化成这些 = 前端缺锚点。

## 3. axe 自动扫描

```js
import AxeBuilder from '@axe-core/playwright';

const results = await new AxeBuilder({ page }).analyze();
const blocking = results.violations.filter(v =>
  ['critical', 'serious'].includes(v.impact));
expect(blocking).toEqual([]);   // 名称缺失、对比度、无 label 等会在此暴露
```

## 4. 逐态验(不能只走 happy path)

error/empty/loading 各只在特定分支出现,单次快照永远到不了。把组件分别驱动进各态(Storybook story / prop 矩阵 / mock 网络),每态都跑 snapshot + locator + 信息文本断言。硬性要求:

```js
// empty 态:必须有可读文本,不能只有插图
await expect(page.getByRole('status')).toHaveText(/未找到|暂无/);
// error 态:进 live region
await expect(page.getByRole('alert')).toBeVisible();
// loading 态:容器 aria-busy
await expect(page.getByTestId('list')).toHaveAttribute('aria-busy', 'true');
```

## 5. 信息型非交互文本也要在树里

计数、状态行、图表 `aria-label`、校验摘要这类信息若被塞进 `title` 属性 / `aria-hidden` 容器 / canvas / CSS `::before`,根本不进树,交互 locator 一个都碰不到。逐处显式断言:

```js
await expect(page.getByText('找到 42 条结果')).toBeVisible();     // StaticText 在树里
await expect(page.getByRole('img', { name: '下降趋势' })).toBeVisible(); // 图表有 name
```

## 6. 状态变化可感知(before/after,不是一次性静态)

live region 靠"变化事件"播报:容器须先空存在,内容注入后才被感知。静态快照看到 alert 节点在就判过,是假通过。

```js
const status = page.getByRole('status');
await expect(status).toBeEmpty();          // 动作前:容器已空存在
await page.getByRole('button', { name: '保存' }).click();
await expect(status).toHaveText('已保存');  // 动作后:出现目标文本
```

## 7. 键盘 + 焦点顺序

```js
// 焦点可见 + Tab 序匹配阅读序 + 无正 tabindex
await expect(page.locator('[tabindex]:not([tabindex="0"]):not([tabindex="-1"])')).toHaveCount(0);
const seq = [];
for (let i = 0; i < N; i++) {
  await page.keyboard.press('Tab');
  seq.push(await page.evaluate(() => {
    const el = document.activeElement;
    return `${el.getAttribute('role') || el.tagName}:${el.textContent?.trim().slice(0,20)}`;
  }));
}
// 断言 seq 匹配预期阅读序;dialog 打开后循环 Tab 断言焦点始终陷在内部
```
- Esc 能关弹窗;打开焦点进内部、关闭归还。复合控件(tablist/listbox)实测方向键能切、只有一个 tabstop。

## 8. motion + 对比度

```js
// 验证前强制 reduced-motion,断言降级路径真被执行(否则该冻结的 spinner 照转、检查静默通过)
await page.emulateMedia({ reducedMotion: 'reduce' });
const anim = await page.locator('.spinner').evaluate(el => getComputedStyle(el).animationDuration);
// 测试/快照构建再全局关动画避免命中时机不稳:
// *{animation-duration:0!important;transition-duration:0!important}
```
- 对比度:正文 ≥ 4.5:1,大字/UI 边界 ≥ 3:1(axe 的 color-contrast 规则会报);且**颜色不是唯一信息通道**(错误配文字/`aria-invalid`,不只红边)。

## 通过标准

- snapshot 里每个可交互/**有信息**元素都有非空 role + name,信息文本不藏在 title/aria-hidden/canvas。
- 关键 locator 全部 `getByRole`/`getByLabel` 唯一解析,无需 `getByTestId` 兜底,也无 css/xpath/哈希。
- idle/loading/error/empty **各态**都验过;状态变化 before/after 可感知。
- axe 0 个 critical/serious;对比度达标、颜色非唯一通道。
- 键盘可遍历、焦点序正确、焦点可见、弹窗焦点陷内部、reduced-motion 降级生效。
