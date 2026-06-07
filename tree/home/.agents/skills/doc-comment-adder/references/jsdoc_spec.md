# JavaScript（纯 JS）JSDoc 文档注释规范

## 定位：JSDoc 在纯 JS 里承担类型职责

本规范面向**纯 JavaScript 文件**（`.js` / `.mjs` / `.cjs`，不含 TypeScript）。

与 `typescript_spec.md` 的根本分工：

* **TS 文件**：类型信息写在签名里（`function f(x: number): string`），编译器即类型真相。JSDoc 只写**行为语义**——做什么、边界、副作用、示例——**不重复类型**，把类型标进注释属于冗余。
* **纯 JS 文件**：语言本身没有类型语法，类型信息**无处可写**。此时 JSDoc 的类型标签（`@type` / `@param {Type}` / `@returns {Type}` / `@typedef`）就是**唯一的类型真相源**，是注释的核心价值，而不是可有可无的补充。

也就是说：在 TS 里「注释别写类型」，在纯 JS 里「注释必须写类型」。这是两份规范最重要的差异，务必先建立这个心智模型再往下读。

工具链层面，JS 的 JSDoc 类型不只是给人看：

* 文件首行写 `// @ts-check`，或在 `tsconfig.json` / `jsconfig.json` 里开启 `checkJs: true`，TypeScript 语言服务就会**把 JSDoc 类型当真**，在 VS Code 等编辑器里提供补全、跳转、类型错误红线——纯 JS 也能拿到接近 TS 的开发体验，且零构建、零语法改动。
* 因此为纯 JS 写 JSDoc 类型，等价于「不引入 TS 工具链的前提下获得静态类型检查」。这是补全 JS 文档注释时最该优先保证的收益。

## 基本形式

* 使用 `/** ... */` 块注释；单行 `//` 与 `/* */` 不被 JSDoc 解析，不能承载标签。
* 文档注释紧贴在被注释声明（函数、类、常量、导出对象、方法、属性）上方，中间不空行。
* 注释正文用 Markdown 组织，支持列表、行内代码与代码块。
* 类型表达式写在花括号里：`{string}`、`{number[]}`、`{Promise<User>}`。这是 JS JSDoc 与 TS TSDoc 写法上最直观的区别——TS 不需要花括号类型。

## 启用类型检查的头注释建议

* 对希望获得编辑器类型检查的纯 JS 文件，建议在**文件第一行**加 `// @ts-check`：

  ```js
  // @ts-check

  /** @type {number} */
  let count = 0;

  count = "x"; // 编辑器在此报类型错误
  ```

* 若项目已在 `jsconfig.json` / `tsconfig.json` 配置 `"checkJs": true`，则全项目默认开启，无需逐文件加 `// @ts-check`；反过来个别文件想关闭可写 `// @ts-nocheck`。
* 局部抑制某行误报用 `// @ts-ignore` 或 `// @ts-expect-error`，仅在确有必要时使用，不要滥用。
* 是否启用以项目既有约定为准：项目已普遍使用 `// @ts-check` 就跟随补齐；项目从不开检查则不要单方面引入，以免触发一片既有红线。

## 类型标注语法全覆盖

### @type：变量、常量、字段的类型

```js
/** @type {string} */
const name = readName();

/** @type {number[]} */
const scores = [];

/** @type {Map<string, number>} */
const counter = new Map();
```

* `@type` 用于当右值无法被推断、或想固定一个更宽/更窄类型时。
* 字面量能被自动推断的，**不要**画蛇添足写 `@type`（见反例一节）。

### @param：参数类型与语义

```js
/**
 * 计算两点间的欧氏距离。
 *
 * @param {number} x1 起点横坐标。
 * @param {number} y1 起点纵坐标。
 * @param {number} x2 终点横坐标。
 * @param {number} y2 终点纵坐标。
 * @returns {number} 两点间距离。
 */
function distance(x1, y1, x2, y2) {
  return Math.hypot(x2 - x1, y2 - y1);
}
```

* 顺序：`{类型}` 在前，参数名次之，描述在后。
* 每个参数都应有类型；描述只在语义不自明时补充。

### @returns：返回值类型

```js
/**
 * 读取配置文件并解析为对象。
 *
 * @param {string} path 配置文件绝对路径。
 * @returns {Promise<Object>} resolve 为解析后的配置对象。
 */
async function loadConfig(path) {
  const text = await fs.readFile(path, "utf8");
  return JSON.parse(text);
}
```

* `async` 函数返回类型写成 `Promise<T>`。
* 返回类型为 `void` / `undefined` 且语义直白时可省略 `@returns`；公共 API、异步、复杂返回值建议保留。

### 可选、默认值、可变、nullable 的写法

```js
/**
 * 格式化用户展示名。
 *
 * @param {string} first 名。
 * @param {string} [last] 姓，可选（等价于 `{string=}`）。
 * @param {string} [sep=" "] 分隔符，默认空格。
 * @param {...string} titles 任意数量的头衔前缀。
 * @returns {string} 拼接后的展示名。
 */
function formatName(first, last, sep = " ", ...titles) {
  // ...
}
```

| 含义 | 写法 A（推荐，方括号） | 写法 B（后缀符号） |
| --- | --- | --- |
| 可选参数 | `@param {string} [last]` | `@param {string=} last` |
| 带默认值 | `@param {string} [sep=" "]` | —— |
| 可变参数 | `@param {...string} titles` | —— |
| 可为 null | `@param {?string} note` | —— |
| 明确非 null | `@param {!Object} cfg` | —— |
| 联合类型 | `@param {string\|number} id` | —— |

* 可选参数优先用方括号 `[name]` 写法，可读性好且能携带默认值，比 `{string=}` 后缀更清晰。
* `{?string}` 表示 `string | null`；`{string=}` 表示参数可省略（与可为 null 是两件事，别混）。
* 联合类型用 `|`：`{"asc"|"desc"}`、`{number|null}`。

### @typedef + @property：定义可复用的对象类型

```js
/**
 * 分页请求配置。
 *
 * @typedef {Object} Paging
 * @property {number} page 当前页码，从 1 起。
 * @property {number} pageSize 每页条数。
 * @property {string} [sort] 排序字段，可选。
 */

/**
 * 规范化分页参数。
 *
 * @param {string} [rawPage] 原始页码字符串。
 * @returns {Paging} 规范化后的分页配置。
 */
function parsePaging(rawPage) {
  // ...
}
```

* `@typedef` 把一个复杂结构命名后，可在本文件任意 `@param` / `@returns` / `@type` 里按名字引用，避免重复书写长结构。
* `@property` 逐字段标注类型与语义，可选字段用 `[name]`。
* 给函数类型起别名也用 `@typedef` 配 `@callback`（见下）。

### @callback：命名回调/函数类型

```js
/**
 * 比较两个元素的回调。
 *
 * @callback Comparator
 * @param {*} a 左元素。
 * @param {*} b 右元素。
 * @returns {number} 负数表示 a 在前，正数表示 b 在前，0 表示相等。
 */

/**
 * 使用比较器排序（原地）。
 *
 * @param {Array<*>} arr 待排序数组。
 * @param {Comparator} cmp 比较器。
 * @returns {Array<*>} 同一数组引用，已排序。
 */
function sortInPlace(arr, cmp) {
  return arr.sort(cmp);
}
```

* `@callback` 等价于「为函数类型定义一个 `@typedef`」，比内联 `{function(a, b): number}` 可读性高，且能复用。

### @template：泛型

```js
/**
 * 从数组按键建立索引映射。
 *
 * @template T 元素类型。
 * @template {PropertyKey} K 索引键类型。
 * @param {T[]} items 数据集合。
 * @param {(item: T) => K} selectKey 提取索引键的函数。
 * @returns {Record<K, T>} 以键为索引的映射。
 */
function indexBy(items, selectKey) {
  const out = /** @type {Record<K, T>} */ ({});
  for (const item of items) out[selectKey(item)] = item;
  return out;
}
```

* `@template T` 声明类型参数，后续 `@param` / `@returns` 即可引用 `T`。
* 带约束写 `@template {PropertyKey} K`，对应 TS 的 `K extends PropertyKey`。
* 函数体内需要类型断言时用行内 `/** @type {X} */ (expr)` 的「cast」语法（注意被断言表达式要用圆括号包住）。

### 内联类型断言（cast）

```js
const el = /** @type {HTMLInputElement} */ (document.getElementById("name"));
el.value = "init";
```

* 当编辑器推断的类型过宽（如 `Element | null`）而你确知更具体类型时，用 `/** @type {T} */ (expr)` 收窄。

## @typedef 提取 vs 内联对象类型的取舍

JSDoc 既能内联写对象类型，也能用 `@typedef` 命名后引用。取舍原则与 `lua_spec.md` 里「内联结构 vs `@class`」是同一套约定：

* **内联**：结构简单、只用一次、字段少（一两个），直接内联，避免为一次性结构造一个全局名字：

  ```js
  /**
   * @param {{x: number, y: number}} point 二维坐标点。
   * @returns {number} 到原点的距离。
   */
  function magnitude(point) {
    return Math.hypot(point.x, point.y);
  }
  ```

* **提取 `@typedef`**：结构被多处复用、字段较多、需要逐字段写语义、或作为公共 API 的输入/输出契约时，提取成命名类型：

  ```js
  /**
   * 几何坐标点。
   * @typedef {{x: number, y: number}} Point
   */
  ```

* 判断口诀：**「用一次且简单 → 内联；用多次或需逐字段解释 → @typedef」**。不要为图省事把会被多处引用的复杂结构反复内联，也不要为只出现一次的 `{a: string}` 强行造 `@typedef`。

## import 类型：跨文件引用类型

```js
/**
 * @param {import('./models/user.js').User} user 用户实体。
 * @returns {string} 用户显示名。
 */
function displayName(user) {
  return user.nickname ?? user.email;
}
```

* `import('模块路径').导出名` 直接在类型位置引用**另一个文件导出的类型**（含该文件用 `@typedef` 定义并导出的类型），无需 `require` / `import` 真正引入值，纯类型引用不产生运行时依赖。
* 路径写法跟随项目模块解析约定（ESM 通常带 `.js` 后缀）。
* 频繁引用同一类型时，可在本文件用 `@typedef` 起本地别名减少重复：

  ```js
  /** @typedef {import('./models/user.js').User} User */
  ```

## 模块级注释：@module 与 ESM / CommonJS

* 在文件顶部用模块级注释概述本文件职责、导出能力与使用边界，至少一行摘要：

  ```js
  /**
   * 用户会话读写与失效处理的公共辅助能力。
   *
   * 集中处理 token 持久化、过期判断与会话恢复流程中的通用逻辑。
   *
   * @module session
   */
  ```

* `@module` 标签声明模块名，常用于生成文档时归类；项目若不依赖 JSDoc 文档生成器，可只写摘要、省略 `@module`。
* **ESM**（`.mjs` 或 `package.json` 设 `"type": "module"`）与 **CommonJS**（`.cjs` / 默认）在文档注释写法上一致；差异只在导出语法本身。给具体导出加注释时，注释贴在该 `export` / `module.exports` 声明上方即可。
* barrel 文件、纯 re-export 文件可省略模块级注释。

## 行为性标签（与 TS spec 共通，简要带过）

纯 JS 同样使用这些**非类型**标签，语义与 `typescript_spec.md` 完全一致，此处不展开，重点仍在类型承载：

* `@throws {Error}`：可能抛出的异常类型与触发条件。
* `@example`：最小可理解的使用示例，代码块用 ```` ```js ````。
* `@deprecated`：弃用原因、起始版本、替代方案与迁移方式，不要只写「已弃用」。
* `@see`：引用相关函数、模块或替代 API。

推荐标签顺序（与 TS 对齐，注意 JS 用 `@template` 而非 `@typeParam`）：摘要 → `@template` → `@param` → `@returns` → `@throws` → `@example` → `@deprecated`。

## 反例

### 反例一：有了类型标签，描述里还复述类型

```js
// ✗ 描述重复了花括号里已写明的类型
/**
 * @param {number} count 一个 number 类型的数字，表示数量。
 * @returns {string} 返回一个 string 字符串。
 */

// ✓ 类型交给花括号，描述只讲语义
/**
 * @param {number} count 商品数量，需为非负整数。
 * @returns {string} 用于界面展示的数量文案。
 */
```

类型已经在 `{number}` 里声明，描述再写「一个 number 类型」是纯噪声。描述只该补充**类型表达不了的语义**：单位、范围、约束、业务含义。

### 反例二：给自明字面量写 @type

```js
// ✗ 右值字面量已能被推断为 number / string，@type 多余
/** @type {number} */
const MAX_RETRY = 3;

/** @type {string} */
const PREFIX = "user:";

// ✓ 直接赋值，让推断生效
const MAX_RETRY = 3;
const PREFIX = "user:";
```

`@type` 应留给**推断不出或需要拓宽/收窄**的场景，例如初始为空但后续会放入特定类型的容器：

```js
// ✓ 空数组推断为 never[]，需显式标注元素类型
/** @type {string[]} */
const names = [];
```

### 反例三：把类型当摘要

```js
// ✗ 摘要只是复述函数名
/** parseConfig 函数。 */

// ✓ 摘要回答「做什么」
/** 解析配置文件文本为运行时配置对象。 */
```

## 风格与边界

* 纯 JS 文件：**优先保证类型标签完整正确**（`@param` / `@returns` / `@type` / `@typedef`），这是注释最大的价值，缺类型的 JSDoc 在纯 JS 里失去了主要意义。
* 类型交给花括号，散文描述只讲语义、边界、副作用、约束，二者不重复。
* 同一文件 / 仓库内统一写法：可选参数统一用 `[name]` 还是 `{T=}`、是否一律写 `@returns`、示例代码块语言标记、`@returns` 还是 `@return`（统一用 `@returns`），保持一致不混用。
* 公共 API、导出函数、跨文件复用的对象结构必须有完整类型注释；纯内部、一次性的私有小函数可按需精简，但只要开了 `// @ts-check`，缺类型就会暴露为推断不足，建议仍补齐关键签名。
* 不要为迎合文档生成器引入项目未约定的标签；以仓库既有标签集合与顺序为准。
