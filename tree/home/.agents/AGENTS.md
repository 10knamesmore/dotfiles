- ~/Documents 是我存放项目的地方
- ~/Documents/repos 是我存放一些remote仓库的地方(需要参考开源项目的时候clone到这里), 一般直接pull到最新的后阅读本地副本, 在这个目录下的一切修改应当被视为可被force 覆盖的,
- ~/dotfiles 是我存放全局 dotfiles 的仓库目录

## 沟通

- 不确定的事不要编造。源码事实通过当前代码查证并提供 path/to/file:line；外部事实引用文档或clone仓库到repos看源码
- 沟通时优先输出可读的 code snippet || unicode art
- 不要用比喻

## 工作方式

- 除非用户显式要求， 否则不要在代码中留下测试, 兜底, 兼容， 防御行为， 如果你认为某个行为需要, 报告并由用户决断
- 优先复用成熟、有人维护的能力，没有明确理由别自己写轮子。方案不明确或需要比较实现路线时，参考成熟产品与开源实现。
- 变量命名, 注释， prose, 类型/api/函数/模块设计 都必须以`human facing`为第一原则， 没有达到`作为0先验知识的人类维护者能一眼看出来大致意思`就是不合格
- 我关注代码美学与架构设计, 生产代码都需注意相关的 naming/模块设计/层次关系/高内聚低耦合(封装边界)/生命周期/底层不要依赖上层
- prefer `rg`/`fd` than `grep` / `find`, 前者尊重gitignore
- when designing frontend, dont use label/desciption/subtitle, let the layout describe it self
