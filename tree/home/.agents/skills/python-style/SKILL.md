---
name: python-style
description: 编写、修改或审查 Python 源码、package 和类型接口时使用；要求类型与运行时行为一致、模块依赖清晰、公开 API 可理解，并通过目标项目自己的质量门槛。
---

# Python Style

## Principles

Python 类型是运行时防错和 human/model-facing 接口，不是只让静态检查器闭嘴的装饰。

- 先遵守目标项目已有的语言版本、依赖、目录结构、配置和验证方式；本 Skill 不指定 package manager、formatter、linter、type checker、validation framework 或测试框架。
- 先检查标准库、现有依赖和项目惯例能否解决问题；没有明确收益不增加依赖或自建基础设施。
- 类型、模块和函数各自表达一个清楚的责任。为了消除诊断而增加无业务含义的层、wrapper 或别名，不算修复。

## Type contract

- 参数、返回值、容器元素和结构化字段写完整类型。不要使用裸容器或让 `Any` 穿透已知边界；闭合集合优先使用 enum、`Literal` 或受约束的领域类型。
- 公开接口避免开放 `**kwargs` 和自由字符串协议。确实承担透明转发或插件入口时，把接受范围、传递对象和失败行为写成明确 contract。
- 公共 annotation 引用的类型应正常导入并能在运行时解析。默认不使用 `TYPE_CHECKING` 或 `from __future__ import annotations` 隐藏依赖、压住循环 import 或推迟缺失符号错误。
- 真正递归的类型可以对递归边使用最小字符串 forward reference。其他 postponed annotation 必须由目标框架的明确要求支持，并在交付说明中给出实际消费者和理由。
- `cast()` 不做运行时检查，不能用来把实际对象伪装成另一个类型或作为处理 `Any` 的默认方法。优先使用窄类型接口、具体返回类型、运行时类型判断或项目已有的边界验证机制。
- 只有源码行为已经证明、但第三方 stub 无法表达同一真实类型时，才允许在最窄边界使用 `cast()`；cast 的目标必须与运行时对象一致，并说明 stub 缺口。不能为了保留实现而把对象 cast 成错误类型。
- 自有 `.py` 源码已有完整类型时，不为可读性另生成重复 `.pyi`；源码是接口 owner。

## Dependency and module design

- 循环依赖首先是 ownership 和 dependency direction 问题。移动 owner、提取真正共享的低层类型、让低层实现泛型化或改成单向依赖；不要用 `TYPE_CHECKING`、函数内 import、字符串 annotation、lazy import 或诊断 suppression 掩盖。
- 公共层拥有公开函数和输入/输出类型，私有基础设施不得为了复用这些类型反向依赖公共层。需要跨层传递时，优先使用真正共享的低层概念、泛型、已验证的基础值或由公共层提供的 adapter。
- 一个模块、类型或函数只做一件事。删除只转发同一组参数的 helper；不要为了 lint 数字制造没有独立语义的 wrapper。

## Public API

- API 以零先验调用者能直接理解为标准：显式参数、明确默认值、具体返回类型、必要的 docstring。内部存储、序列化和传输细节不进入公开签名。
- 参数多但各自都是调用者独立决策时，保留显式签名。若项目规则与真实公共接口冲突，修正规则或在 owning configuration 中记录一次理由；不要逐函数 suppression，也不要凭空发明 `XxxInput`。
- 只有输入本身是可复用、嵌套或有独立不变量的结构时才建立 input model。固定取值、互斥关系、日期时间语义和单位尽量进入类型或 validator。

## Runtime boundaries

- 外部数据、配置、模型输入、网络请求和响应等非受信结构在进入业务代码时验证，并归一成项目自己的具体类型；不让未解析的异构映射穿透边界。
- 输入是否允许未知字段、响应如何处理新增字段、失败使用什么异常，由真实兼容契约和 consuming framework 决定，并在 owning boundary 明确表达；不使用全局宽松转换。
- Validator 或 parser 应抛出其 consuming framework 会正确归一和展示的异常。不要为了满足通用规则而改变实际错误边界。
- 日期时间必须明确 naive/aware 和实际时区语义。不要给上游缺失的 timezone 编造含义，也不要无理由丢掉已有 timezone。

## Ignore and gate policy

- 默认不新增 inline ignore、baseline、排除路径或全局规则放宽。先修类型、依赖方向和 owner。
- 确有必要保留时，逐项列出 rule code、实际命中对象、为什么正确代码仍会命中、替代方案为何会损害真实 contract。只写“第三方问题”“Python 限制”或“兼容需要”不算理由。
- 全局 ignore 只能表达整个 package 一致的规则选择；单点例外保持单点。配置前先审计命中数量和类别，删除无命中、已过期或只为历史实现存在的项。
- 自有 package 的质量门槛应覆盖循环依赖、无效 suppression、未知类型和 `Any` 传播。不要因为某个工具声称处于 strict 模式，就假设这些风险都已覆盖；检查目标项目实际启用的规则。

## Verification

使用目标项目已经配置并锁定的 formatter、linter、type checker 和其他质量入口，对项目要求的完整 scope 验证，不在全局 Skill 中替项目选择工具或命令。所有启用的 error 和 warning 都必须 clean；失败时修代码或结构，不通过降低门槛交付。

修改 runtime annotations、结构化边界或 package import graph 时，按风险验证真实消费者：实际 import、运行时 annotation 解析、边界模型构造或最窄运行时 probe。交付前检查目标 scope 中的 deferred annotations、static-only imports、`cast`、inline ignore、baseline、排除项和 cycle gate，并说明每个保留项。
