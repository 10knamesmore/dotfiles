- ~/Documents 是我存放编程项目的地方
- ~/Documents/repos 是我存放一些remote仓库的地方(任何时候我/你需要参考开源项目的时候clone到这里)
- ~/dotfiles 是我存放全局 dotfiles 的manager 仓库目录

## 沟通

- 直说结论，不谄媚不奉承；发现我说错了直接指出，别顺着我
- 不确定的事明说不确定，不要编造 API/字段/路径——查证优先于回忆, 任何观点/事实 都必须通过查证, 源码 clone下来读/文档/websearch, 并提供 path/to/file:line
- 任何非平凡改动前先对齐方案；
- 任何时候提到代码原文, 使用 path/to/file:line 说明清楚格式

## 工作方式

- **禁止** 自主编写任何测试， 不管是单元/集成/E2E, 除非用户明确要求
- **禁止** 自主考虑兜底/兼容， 除非用户明确要求
- 当用户对话中途插入其他信息时， 默认接受而不是转换目标，大多数情况下用户需要的是多目标并行
- **禁止** 代码/注释/面对用户的文本 等地方使用日文引号
- 声明"完成 / 修好 / 通过"前先实际验证（跑命令看输出）；没验证的步骤如实说，失败不粉饰
- 注释与其他持久 prose 使用 `prose-standard`；编写过程泄漏使用 `trim-cot-leakage`
- 任何一个功能,先翻项目里已有的依赖能做什么, 优先用成熟的、有人维护的库。没有明确理由别自己写轮子。
- 架构决策往长了做。不接受"先这样以后再换"的临时方案。
- 先看成熟产品怎么解决同一个问题，用已验证的模式，别从零发明。
- 任何时候， 高内聚低耦合, 比如能模块层次结构化的就不要平铺, 能用子结构体的就不要平铺, 任何模块/struct/函数, 反思是否遵循了`do one thing`原则, 如果不是是否有充足的理由

```text
// 不要
x.rs
x_a.rs
x_b.rs
// 要
x/a.rs
x/b.rs

不要
struct A {x,y,z_a,z_b,z_c}
要
struct Z {a,b,c}

struct A {x,y,Z}
```

## 工具偏好

下文提到的工具都已经安装， 可以直接使用
标"hook"的是硬约束（在支持 hook 拦截的 harness 里用错会被拦回让你换）；其余是建议。

**搜索 / 导航**

- rg 替代 grep (hook)。易错:`-r` 是 `--replace` 不是 recursive(递归本就是默认),千万别写 `rg -rn`(=`-r n`,把每个匹配替换成字面 `n`、还没行号)——要行号就 `rg -n`;默认跳过 .gitignore/hidden(.git、target 等),搜全用 `-uu`;pattern 是正则,字面量用 `-F`
- `fd` 替代 `find`（hook）。易错:pattern 是正则、只匹**文件名**子串,要 glob 用 `-g`、匹完整路径用 `-p`、字面量 `-F`;默认跳过 .gitignore/hidden,搜全用 `-u`(=`--no-ignore --hidden`);`-e` 扩展名、`-t f/d` 类型
- `ast-grep` 做结构化代码搜索 / 替换：按 AST 匹配，重构远胜 grep/sed。易错:二进制用 `ast-grep`(`sg` 撞系统命令);pattern 是**代码片段非正则**,metavar 必**大写**——`$VAR` 单节点、`$$$VAR` 变长(参数列表等)、`$_` 不捕获;`-l` 指语言(默认按扩展名);`-r/--rewrite` 默认只**预览 diff**,加 `-U`(全改)/`-i`(交互)才写盘;规则式扫描走 `ast-grep scan`(YAML)

**语言工具链**

- Python 一律 `uv`（`uv run` / `uv add`），不用 pip/python（hook）
- Rust 测试用 `cargo nextest`，配 `cargo clippy` / `cargo fmt`
- JS（若碰）用 `pnpm` 替代 npm（hook）

- GitHub 操作一律 `gh` CLI，深入研究仓库 clone 到 /tmp，不用 WebFetch（hook 拦 github 域）
