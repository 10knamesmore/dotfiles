- ~/Documents 是我存放项目的地方
- ~/Documents/repos 是我存放一些remote仓库的地方(需要参考开源项目的时候clone到这里), 一般直接pull到最新的后阅读本地副本, 在这个目录下的一切修改应当被视为可被force 覆盖的,
- ~/dotfiles 是我存放全局 dotfiles 的仓库目录

## 沟通

- 不确定的事不要编造。源码事实通过当前代码查证并提供 path/to/file:line；外部事实引用文档或clone仓库到repos看源码
- 沟通时优先输出可读的 code snippet || unicode art
- 不要用比喻
- Be thorough in your analysis and bold in your recommendations

## 工作方式

- 东西做出来能工作 > 安全 > 在corner case下的可靠， 连第一点都满足不了， 去追求后面完全是舍本逐末, 所以在没保证业务功能整整实现前禁止防御
- 当你做了A, 我认为应该是做B的时候
    - 不要: 把A改成B, 然后留下注释`这里是B, 非A`
    - 应该: B 就当成自然的最后状态
- 优先复用成熟、有人维护的能力，没有明确理由别自己写轮子。方案不明确或需要比较实现路线时，参考成熟产品与开源实现。
- 变量命名, 注释， prose, 类型/api/函数/模块设计 都必须以`human facing`为第一原则， 没有达到`作为0先验知识的人类维护者能一眼看出来大致意思`就是不合格, `概念的准确性`不应高于`易于理解`
- 我关注代码美学与架构设计, 生产代码都需注意相关的 naming/模块设计/层次关系/高内聚低耦合(封装边界)/生命周期/底层不要依赖上层
- prefer `rg`/`fd` than `grep` / `find`, 前者尊重gitignore
- when designing frontend, dont use label/desciption/subtitle, let the layout describe it self
- 遇到锁的时候想想channel 能不能解决, 遇到 lease/registry/semaphore 的时候想想one shotchannel能不能解决
- 遇到面向对象, 用self 字段/锁表达状态的时候, 考虑怎么过程式/声明式/函数式 用channel改造, 让一切更直观
- 好的解决方案应当是简洁/直观/易于理解的
    - 兜底, 兼容， 防御行为， 当你认为某个边界情况可能需要考虑的时候, 先真正看代码, 考虑清楚这些情况是否真的会发发生. 如果会, 优先考虑的是应该怎么设计让他理论上就不可能发生, 而不是添加什么后天机制去防御这些情况, 如果确实需要， 先和用户讨论
    - 对于测试，应该用集成/e2e测试， 把一个模块当成黑盒， 而不是照着代码写答案， 这种单元测试没有意义
    - 对于业务/代码 逻辑, should be easy to get things right, hard to get things wrong
    - 抽象应该尽可能少, 好的设计应当天生就让不合法情况不会发生, 而不是叠加机制/抽象去做额外的防御
