# 验证 a11y locator contract

只验证 component semantics/state 是否已标注为 test 可直接 assert 的 public a11y contract，并覆盖组件实际支持的每个 UI state。不安装新的测试框架，不建立业务测试，不触发完整业务流程，不断言业务结果。

## Accessibility snapshot

使用项目已有的 Playwright 或 CDP 环境读取目标所在 subtree：

```ts
const snapshot = await page.locator('main').ariaSnapshot()
console.log(snapshot)
```

只检查当前任务的目标是否具有正确且非空的 role 和 accessible name。

CDP 环境可读取完整 AX tree：

```ts
const client = await page.context().newCDPSession(page)
await client.send('Accessibility.enable')
const { nodes } = await client.send('Accessibility.getFullAXTree')
```

## Strict uniqueness

对目标执行唯一性断言，不需要 click、fill、submit 或等待业务结果：

```ts
await expect(page.getByRole('button', { name: '保存' })).toHaveCount(1)
await expect(page.getByLabel('邮箱')).toHaveCount(1)
```

有重复名称时验证语义 scope：

```ts
const dialog = page.getByRole('dialog', { name: '确认删除' })
await expect(dialog).toHaveCount(1)
await expect(dialog.getByRole('button', { name: '确认' })).toHaveCount(1)
```

需要 state 区分当前元素时，只验证 locator 解析：

```ts
await expect(
  page.getByRole('tab', { name: '通用', selected: true }),
).toHaveCount(1)
```

## State matrix

使用已有 Story、component props 或最小 state harness，直接把组件渲染到它实际支持的每个状态。不要为了进入状态搭建完整后端、账号或业务数据。

| State | A11y verification |
|---|---|
| idle | 主要目标 role/name 唯一 |
| loading | 原目标 name 稳定；busy region、progressbar 或 status 唯一可定位 |
| success | success status 或结果区域唯一可定位 |
| error | alert、invalid field 或错误区域唯一可定位 |
| empty | empty status 或命名区域唯一可定位 |

示例只断言 a11y projection：

```ts
const list = page.getByRole('region', { name: '订单列表' })
await expect(list).toHaveAttribute('aria-busy', 'true')
await expect(
  list.getByRole('status', { name: '订单加载状态' }),
).toHaveCount(1)

await expect(
  page.getByRole('alert', { name: '保存错误' }),
).toHaveCount(1)

await expect(
  page.getByRole('status', { name: '搜索结果状态' }),
).toHaveCount(1)
```

这里不验证请求是否发出、保存是否真实成功、错误码是否正确或查询为什么为空。

## 失败处理

- 命中 0 个：检查目标是否进入 accessibility tree、role 是否正确、name 是否为空。
- 命中多个：补充真实上下文，或使用有名字的语义容器收窄。
- 只能用 `data-testid`、CSS 或 XPath：修复 role、name 或 scope，不把 fallback 当作通过。
- locator 指向 detached node：检查不稳定 key、节点重建、portal、virtual list 或 shadow boundary。

## 通过标准

- 当前任务的目标出现在 accessibility snapshot 中。
- role 与 accessible name 正确、稳定。
- `getByRole` 或 `getByLabel` 在合理 scope 内恰好命中一个节点。
- 组件实际支持的每个 UI state 都有对应的、唯一可定位的 a11y representation。
- 验证过程没有引入或执行任何业务级测试。
