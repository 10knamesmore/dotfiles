# TypeScript 文档注释规范（TSDoc / JSDoc 风格）

## 基本形式

* 使用 `/** ... */` 作为文档注释。
* 文档注释适用于导出的函数、类、接口、类型别名、枚举、常量、Hook、React 组件以及公共方法。
* 注释内容使用 Markdown 风格组织，支持列表、代码块与行内代码。
* 优先采用与项目现有工具链兼容的风格；若仓库已约定使用 TSDoc 或 JSDoc 的特定标签集合，应保持一致。

## 放置位置

* 文档注释应紧贴在被注释声明上方。
* 不要在无实际信息增量的场景下重复注释，例如类型别名、接口字段与实现代码已经足够直接表达语义时，不强行补充空洞描述。
* 对于类成员或对象方法，优先为公共 API、关键业务逻辑、边界行为明显的方法补充文档；纯内部私有实现可按需要省略。
* 对于 React 组件、Hook 和导出的工具函数，原则上应提供文档注释。

## 内容建议

* 第一行使用简短、完整的句子概述用途，直接说明“这个声明做什么”。
* 当参数、返回值、错误、约束、副作用、默认行为或使用方式不直观时，使用标签补充。
* 优先描述调用者需要知道的行为、边界、约束与返回语义，而不是解释实现细节。
* 示例应与真实函数签名、类型名、组件名和调用方式一致。

## 常用标签

* `@param`：说明参数语义、单位、范围、默认值、是否可选、与其他参数的关系。
* `@returns`：说明返回值的含义、结构与特殊返回情况。
* `@throws`：说明可能抛出的异常及触发条件。
* `@example`：给出最小可运行或最小可理解的使用示例。
* `@deprecated`：说明弃用原因、迁移建议与替代方案。
* `@remarks`：补充行为细节、边界条件、兼容性说明、性能特征或副作用。
* `@typeParam`：说明泛型参数的语义与约束。
* `@defaultValue`：说明默认值，适用于属性、配置项或可选参数。
* `@see`：引用相关类型、函数、模块或替代 API。
* `@public`、`@internal`、`@privateRemarks`：仅在项目工具链明确支持并已采用时使用，不额外引入未经项目约定的标签。

## 自定义模板约定

* 当仓库采用自定义 TypeScript 注释模板时，优先遵循项目内既有标签、顺序与格式约定，保持生成结果一致。
* 若项目已经稳定使用 `@param` / `@returns` / `@remarks` 风格，就不要混入另一套自定义段落式写法。
* 若项目在 React 场景下习惯对 `Props` 接口写总说明、对组件本体只写摘要，则继续沿用，不随意切换为另一种结构。
* 同一仓库内统一使用同一组标签命名，不混用 `@return` 与 `@returns` 等不同写法；通常优先使用 `@returns`。

## 函数注释结构

* 第一行写一句简短摘要，说明函数做什么。
* 当函数参数或返回值语义不直观时，补充对应标签。
* 推荐顺序如下：

  * 摘要
  * `@typeParam`
  * `@param`
  * `@returns`
  * `@throws`
  * `@example`
  * `@remarks`
  * `@deprecated`
* 无参数函数可省略 `@param`。
* 返回值语义非常直接时，可省略 `@returns`；但公共 API、异步函数、复杂对象返回值建议保留。
* 不抛异常或项目风格不强调异常说明时，可省略 `@throws`。
* `@example` 中使用 `ts`、`tsx` 或 `typescript` 代码块，示例需与真实 API 一致。
* `@remarks` 用于补充副作用、缓存行为、幂等性、异步时序、空值策略、性能特征、兼容性限制等；无额外信息时可省略。

## 类型注释结构

* 类、接口、类型别名、枚举的第一行先写摘要。
* 当类型存在关键字段、判别属性、约束关系或使用前提时，应在摘要之后补充说明。
* 接口或对象类型中，关键字段可直接在字段上方单独写文档注释；若需要说明整体语义，也可在类型本身添加总说明。
* 推荐内容包括：

  * 摘要
  * 泛型参数说明（如需要）
  * 关键字段或判别条件说明
  * `@example`
  * `@remarks`
* 对于枚举，可说明每个成员的业务含义；对字面量联合类型，可说明各取值的语义边界。

## React 组件注释结构

* 每个导出的 React 组件都应有文档注释。
* 第一行说明组件用途与主要交互目标，而不是仅复述组件名称。
* 对于组件的 `Props`：

  * 可在 `Props` 接口或类型别名上说明整体语义；
  * 对关键字段单独补充注释；
  * 在组件定义本体上使用 `@param props` 或直接说明组件接收的关键输入。
* 推荐顺序如下：

  * 摘要
  * `@param`
  * `@returns`
  * `@example`
  * `@remarks`
* 当组件存在受控/非受控两种模式、异步加载、事件回调时序、渲染条件、副作用或性能注意事项时，应在 `@remarks` 中说明。
* 示例应尽量最小化，只展示典型使用方式，避免贴出过长 JSX 模板。

## Hook 注释结构

* 每个导出的 Hook 都应有文档注释。
* 摘要应说明 Hook 提供的状态管理、派生值、异步流程或行为封装能力。
* 当 Hook 接收配置对象、返回复合对象，或具有副作用、依赖项约束、调用时序要求时，必须补充说明。
* 推荐顺序如下：

  * 摘要
  * `@typeParam`
  * `@param`
  * `@returns`
  * `@throws`
  * `@example`
  * `@remarks`
* `@remarks` 中可说明：

  * 必须在组件顶层调用；
  * 依赖项变化时的行为；
  * 是否触发请求、订阅、缓存、清理逻辑；
  * 返回函数或状态字段的稳定性约束。

## 类与方法注释结构

* 类文档先说明该类的职责、生命周期定位或使用场景。
* 公共方法应说明其行为、输入输出与副作用。
* 构造函数仅在初始化规则、默认行为或约束不明显时补充注释。
* 对状态型类，可通过类级注释统一解释整体职责，在方法上只写独特行为，不重复通用背景。

## 字段与属性注释

* 对配置对象字段、接口属性、类公共属性、上下文对象字段、返回对象中的关键字段，可直接使用文档注释说明。
* 字段说明应关注职责、单位、约束、可选条件、默认值及业务意义。
* 对明显的布尔开关、状态位、时间戳、分页参数、缓存键、标识符字段，建议补充说明，避免歧义。

## 文件与模块级注释

* 在需要说明模块职责、边界或导出内容时，可在文件顶部添加模块级文档注释。
* 文件级注释至少包含一行摘要；当模块职责不明显时，可补充说明其提供的能力、依赖关系与使用边界。
* 文件级注释应概括模块整体用途，而不是重复罗列内部每个实现细节。
* 对 barrel 文件、纯 re-export 文件或极简单的类型声明文件，可按需省略。

## 泛型与复杂类型说明

* 当函数、类、接口存在泛型参数时，若泛型名称本身不足以表达语义，应使用 `@typeParam` 补充说明。
* 当多个泛型之间存在约束关系、协变/逆变语义、映射关系或判别逻辑时，应明确说明。
* 对复杂返回类型、条件类型、联合类型与判别联合，优先描述“调用者如何理解这个类型”，而不是展开其内部实现机制。

## 异步与错误说明

* 对 `async` 函数，应说明 Promise resolve 的值语义，以及 reject 或抛错的主要条件。
* 若函数内部吞掉异常、返回兜底值、采用重试或降级策略，应在 `@remarks` 或 `@throws` 中说明。
* 若 API 不抛异常，而是通过返回对象中的状态字段表达失败，也应在 `@returns` 中明确。

## 弃用说明

* 使用 `@deprecated` 时，应同时说明：

  * 为什么弃用；
  * 从哪个版本开始不推荐使用；
  * 推荐替代方案是什么；
  * 调用方应如何迁移。
* 不要只写“已弃用”而不提供上下文。

## 示例规范

* `@example` 示例应当最小、真实、可读。
* 示例优先展示典型调用路径，而不是覆盖所有边界条件。
* 示例必须与当前真实签名一致，不得引用不存在的字段、参数或返回结构。
* 对 React 组件使用 `tsx` 示例，对普通 TypeScript API 使用 `ts` 或 `typescript` 示例。
* 示例中可适度省略无关上下文，但不能让调用方式失真。

## 风格与边界

* 避免仅复述函数名、类型名或组件名。
* 摘要优先回答“它做什么”，而不是“它是什么”。
* 保持同一文件与同一仓库内风格一致，包括：

  * 是否总是写 `@returns`
  * 是否为简单参数补 `@param`
  * 示例代码块语言标记
  * 空行与换行风格
* 不要引入与调用者无关的实现细节，例如内部循环方式、临时变量组织方式、私有缓存结构等。
* 公共 API 应有文档注释；内部实现按需要补充。
* 当项目模板要求固定标签顺序时，补全文档时保持该顺序，不随意重排。

## TypeScript 代码块示例

### 函数注释示例

````ts
/**
 * 将原始查询参数解析为分页请求配置。
 *
 * @param rawPage 原始页码字符串，使用 1 作为起始页。
 * @param rawPageSize 原始每页数量字符串。
 * @returns 规范化后的分页配置对象。
 * @throws {Error} 当页码或每页数量不是合法正整数时抛出异常。
 * @example
 * ```ts
 * const paging = parsePaging("2", "20");
 * console.log(paging.page); // 2
 * console.log(paging.pageSize); // 20
 * ```
 * @remarks
 * 缺失输入时会回退到默认分页参数。
 */
export function parsePaging(rawPage?: string, rawPageSize?: string): {
  page: number;
  pageSize: number;
} {
  const page = Number(rawPage ?? 1);
  const pageSize = Number(rawPageSize ?? 20);

  if (!Number.isInteger(page) || page <= 0) {
    throw new Error("Invalid page");
  }

  if (!Number.isInteger(pageSize) || pageSize <= 0) {
    throw new Error("Invalid pageSize");
  }

  return { page, pageSize };
}
````

### 泛型函数示例

````ts
/**
 * 从数组中按选择器提取键并建立索引映射。
 *
 * @typeParam T 数组元素类型。
 * @typeParam K 索引键类型。
 * @param items 待建立索引的数据集合。
 * @param selectKey 从元素中提取索引键的函数。
 * @returns 以键为索引的映射对象。
 * @example
 * ```ts
 * const map = indexBy(
 *   [{ id: "a", name: "Alice" }, { id: "b", name: "Bob" }],
 *   (item) => item.id,
 * );
 *
 * console.log(map.a.name); // Alice
 * ```
 */
export function indexBy<T, K extends PropertyKey>(
  items: T[],
  selectKey: (item: T) => K,
): Record<K, T> {
  return items.reduce((acc, item) => {
    acc[selectKey(item)] = item;
    return acc;
  }, {} as Record<K, T>);
}
````

### 类型注释示例

```ts
/**
 * 描述任务调度器运行时使用的配置项。
 *
 * @remarks
 * 该配置对象通常由环境变量解析结果与默认值合并得到。
 */
export interface SchedulerConfig {
  /** 调度器实例名称，用于日志与指标上报。 */
  name: string;

  /** 单个任务失败后的最大重试次数。 */
  retryLimit: number;

  /** 轮询间隔，单位为毫秒。 */
  pollIntervalMs: number;
}
```

### React 组件示例

````tsx
/**
 * 显示用户摘要信息卡片。
 *
 * @param props 组件属性。
 * @param props.name 用户展示名称。
 * @param props.email 用户邮箱地址。
 * @param props.onSelect 点击卡片后的回调。
 * @returns 用户信息卡片节点。
 * @example
 * ```tsx
 * <UserCard
 *   name="Alice"
 *   email="alice@example.com"
 *   onSelect={() => console.log("selected")}
 * />
 * ```
 * @remarks
 * 当 `onSelect` 未传入时，卡片仅展示信息，不响应点击交互。
 */
export function UserCard(props: {
  name: string;
  email: string;
  onSelect?: () => void;
}) {
  const { name, email, onSelect } = props;

  return (
    <button type="button" onClick={onSelect}>
      <div>{name}</div>
      <div>{email}</div>
    </button>
  );
}
````

### Hook 示例

````ts
/**
 * 管理布尔开关状态并提供快捷操作函数。
 *
 * @param initialValue 初始状态，默认为 `false`。
 * @returns 当前状态以及打开、关闭、切换方法。
 * @example
 * ```ts
 * const { value, open, close, toggle } = useToggle();
 * toggle();
 * console.log(value);
 * ```
 */
export function useToggle(initialValue = false) {
  const [value, setValue] = useState(initialValue);

  return {
    value,
    open: () => setValue(true),
    close: () => setValue(false),
    toggle: () => setValue((current) => !current),
  };
}
````

### 文件级注释示例

```ts
/**
 * 提供用户会话读取、写入与失效处理的公共辅助能力。
 *
 * 该模块集中处理 token 持久化、过期判断以及会话恢复流程中
 * 使用的通用逻辑。
 */
```
