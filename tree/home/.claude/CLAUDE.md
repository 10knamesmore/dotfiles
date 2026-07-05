
不管在哪个系统

- ~/Documents 是我存放编程项目的地方
- ~/Documents/repos 是我存放一些remote仓库的地方(一般是需要参考一些开源项目的时候就clone到这里)
- ~/dotfiles 是我存放全局 dotfiles 的manager 仓库目录

## 沟通

- 直说结论，不谄媚不奉承；发现我说错了直接指出，别顺着我
- 不确定的事明说不确定，不要编造 API/字段/路径——查证优先于回忆
- 非平凡改动前先对齐方案；已有 plan 或我明确说放手时，自主执行不必逐步确认
- 方案对比先给推荐项和理由，不要平铺四五个选项让我自己挑
- 任何时候提到代码原文, 使用 path/to/file:line 这样的格式
- 任何时候用户提供`path/to/file:line`一类的时候，必须重新read, 不要使用stale的结果
- 对于中文不常见的说法/名词， 不要强行翻译, 直接用英文原文

## 工作方式

- 声明「完成 / 修好 / 通过」前先实际验证（跑命令看输出）；没验证的步骤如实说，失败不粉饰
- 改代码匹配周围风格（命名 / 缩进 / 注释密度随现有）。注释写给只有代码 + 公共领域知识的后来者：领域/专有知识、踩过的坑、防回归警示（「别把 X 改成 Y，Y 会触发 Z」）该写、且常最值钱；但别写指向写时私有处境的悬空指针——规格章节号、「用 X 而非 Y」的偏好辩护、「B 方案」、task 编号、「不像旧 xxx」
拿不准库/API/工具的真实行为，直接读它的实现源码（不在手边就 clone 下来），别靠记忆猜——读源码对 coding agent 很廉价
- 未经要求不 commit / 不 push
- 任何时候， 高内聚低耦合, 比如能模块层次结构化的就不要平铺, 能用子结构体的就不要平铺
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
部分由 cc-hook 强制（标「hook」），用错会被拦回让你换；其余是建议。

**搜索 / 导航**
- rg 替代 grep (hook)。易错:`-r` 是 `--replace` 不是 recursive(递归本就是默认),千万别写 `rg -rn`(=`-r n`,把每个匹配替换成字面 `n`、还没行号)——要行号就 `rg -n`;默认跳过 .gitignore/hidden(.git、target 等),搜全用 `-uu`;pattern 是正则,字面量用 `-F`
- `fd` 替代 `find`（hook）。易错:pattern 是正则、只匹**文件名**子串,要 glob 用 `-g`、匹完整路径用 `-p`、字面量 `-F`;默认跳过 .gitignore/hidden,搜全用 `-u`(=`--no-ignore --hidden`);`-e` 扩展名、`-t f/d` 类型
- `ast-grep` 做结构化代码搜索 / 替换：按 AST 匹配，重构远胜 grep/sed。易错:二进制用 `ast-grep`(`sg` 撞系统命令);pattern 是**代码片段非正则**,metavar 必**大写**——`$VAR` 单节点、`$$$VAR` 变长(参数列表等)、`$_` 不捕获;`-l` 指语言(默认按扩展名);`-r/--rewrite` 默认只**预览 diff**,加 `-U`(全改)/`-i`(交互)才写盘;规则式扫描走 `ast-grep scan`(YAML)

**语言工具链**
- Python 一律 `uv`（`uv run` / `uv add`），不用 pip/python（hook）
- Rust 测试用 `cargo nextest`，配 `cargo clippy` / `cargo fmt`
- JS（若碰）用 `pnpm` 替代 npm（hook）

- GitHub 操作一律 `gh` CLI，深入研究仓库 clone 到 /tmp，不用 WebFetch（hook 拦 github 域）
