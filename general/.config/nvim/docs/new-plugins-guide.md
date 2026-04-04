# 新增插件使用指南

重启 Neovim 后 Lazy 会自动安装这 4 个插件。

---

## 1. nvim-surround — 包围符号操作

操作包围符号（括号、引号、HTML 标签等）的三个核心动作：

### 添加包围：`ys{motion}{char}`

`ys` = "you surround"，后接一个动作（text object / motion）和一个包围字符。

```
光标在 hello 上：
  ysiw"     →  "hello"          -- iw = inner word
  ysiw)     →  (hello)          -- ) 无空格，( 有空格
  ysiw(     →  ( hello )        -- 注意区别
  ysiw}     →  {hello}
  ysiw{     →  { hello }
  ysiw<div> →  <div>hello</div> -- HTML 标签

  yss"      →  "整行内容"        -- yss = 包围整行
  yS{motion}{char}              -- 包围并将内容放到新行（带缩进）
```

**Visual 模式**：选中文本后按 `gs{char}` 即可包围选区（`S` 已用于 Flash treesitter）。

```
选中 hello world 后：
  gs"   →  "hello world"
  gs)   →  (hello world)
  gs<p> →  <p>hello world</p>
```

### 修改包围：`cs{old}{new}`

`cs` = "change surround"

```
"hello"    cs"'    →  'hello'        -- 双引号换单引号
'hello'    cs'`    →  `hello`        -- 单引号换反引号
(hello)    cs)]    →  [hello]        -- 圆括号换方括号
(hello)    cs)<div>→  <div>hello</div>
<div>hello</div>   cst"  →  "hello"  -- t = tag，任意标签
```

### 删除包围：`ds{char}`

`ds` = "delete surround"

```
"hello"      ds"   →  hello
(hello)      ds)   →  hello
{hello}      ds}   →  hello
<div>hello</div>  dst  →  hello      -- t = 删除最近的标签
```

### 常用场景速查

| 场景               | 按键          | 效果                            |
| ------------------ | ------------- | ------------------------------- |
| 给变量加引号       | `ysiw"`       | `name` → `"name"`               |
| 字符串换引号类型   | `cs"'`        | `"str"` → `'str'`               |
| 去掉函数调用的括号 | `dsf`         | `func(arg)` → `arg`             |
| 给选区加括号       | Visual + `gs)` | `x + y` → `(x + y)`            |
| 给整行加花括号     | `yss{`        | `return x` → `{ return x }`     |
| 包裹成 HTML 标签   | `ysiw<span>`  | `text` → `<span>text</span>`    |
| 改 HTML 标签       | `csth1`       | `<p>text</p>` → `<h1>text</h1>` |

### 与 text object 组合

surround 的强大之处在于可以与任何 motion/text object 组合：

```
ys2aw"    给光标后 2 个单词加引号
ysap)     给整个段落加括号
ysif"     给整个函数内部加引号（配合 mini.ai 的 f text object）
```

---

## 2. nvim-spider — 子词移动

让 `w`/`e`/`b` 识别 camelCase、snake_case、SCREAMING_SNAKE 等子词边界。

### 默认 vim 的 `w` 行为

```
myVariableName
^                  -- 按 w 一次直接跳到下一个词
```

### spider 的 `w` 行为

```
myVariableName
^  ^       ^       -- 按 w 分 3 次停：my|Variable|Name
```

### 各种命名风格

```
camelCase:          camel|Case
snake_case:         snake|_|case
SCREAMING_SNAKE:    SCREAMING|_|SNAKE
kebab-case:         kebab|-|case
dot.notation:       dot|.|notation
```

### 按键说明

| 按键 | 作用                    |
| ---- | ----------------------- |
| `w`  | 跳到下一个子词开头      |
| `e`  | 跳到当前/下一个子词结尾 |
| `b`  | 跳到上一个子词开头      |

这些键在 **Normal、Visual、Operator-pending** 模式都生效，意味着 `dw`、`cw`、`yw` 等操作也会按子词工作。

### 如果不想要子词行为

按大写的 `W`/`E`/`B` 仍然是 Vim 默认的 WORD 移动（以空格分隔），不受 spider 影响。

---

## 3. telescope-undo — Undo 历史浏览

在 Telescope 中可视化浏览 undo 树的所有分支，预览每个节点的 diff。

### 打开

```
<leader>su    打开 Undo History picker
```

### Telescope 内操作

| 按键     | 模式          | 作用                                         |
| -------- | ------------- | -------------------------------------------- |
| `<CR>`   | Normal/Insert | **恢复**到选中的 undo 状态（yank additions） |
| `<S-CR>` | Normal/Insert | **恢复**到选中的 undo 状态（yank deletions） |
| `<C-CR>` | Normal/Insert | **恢复**到选中的 undo 状态（不 yank）        |

### 使用场景

1. **找回删除的代码**：打开 `<leader>su`，在 diff 预览中找到包含你想恢复的代码的那个状态，回车恢复
2. **浏览修改历史**：不记得改了什么？用 undo 历史逐个查看每次修改的 diff
3. **undo 分支**：Vim 的 undo 是树状结构（不是线性的），`u` 只能线性回退，但 telescope-undo 能看到所有分支

### 什么是 undo 分支？

```
状态A → 状态B → 状态C    (你 undo 回到 B)
                  ↓
              状态B → 状态D    (然后做了新修改)
```

普通 `u`/`<C-r>` 只能在当前分支移动，状态C 就丢失了。
telescope-undo 能看到所有分支，包括状态C，可以跳回去。

---

## 4. nvim-bqf — Quickfix 增强

增强 quickfix 窗口，添加文件预览和操作能力。当你使用 `:grep`、`:make`、`gr`（LSP references）等产生 quickfix 列表时会自动生效。

### 自动生效

打开 quickfix 窗口（`:copen`）时 bqf 自动激活，你会看到：

- 右侧出现**预览窗口**，显示当前选中条目的文件内容和高亮行
- 移动光标时预览自动更新

### Quickfix 窗口内的按键

| 按键    | 作用                                       |
| ------- | ------------------------------------------ |
| `o`     | 在之前的窗口中打开条目                     |
| `O`     | 在之前的窗口中打开条目并关闭 quickfix      |
| `p`     | 切换预览窗口的打开/关闭                    |
| `<Tab>` | 标记/取消标记条目，可多选后操作            |
| `zn`    | 用标记的条目创建新的 quickfix 列表         |
| `zN`    | 用**未**标记的条目创建新的 quickfix 列表   |
| `<C-f>` | 在 quickfix 列表中**模糊搜索**（fzf 过滤） |
| `<C-t>` | 在新 tab 中打开条目                        |
| `<C-v>` | 在垂直分屏中打开条目                       |
| `<C-x>` | 在水平分屏中打开条目                       |

### 典型工作流

**1) Grep 后筛选结果：**

```
<leader>/            -- live_grep 搜索
                     -- 结果太多？发送到 quickfix
                     -- 在 quickfix 中按 <C-f> 二次过滤
                     -- Tab 标记需要的条目
                     -- zn 只保留标记的条目
```

**2) LSP references 逐个查看：**

```
gr                   -- 查看引用，结果进入 quickfix
:copen               -- 打开 quickfix
j/k                  -- 上下移动，右侧预览自动跟随
<CR>                 -- 跳转到选中的引用
```

**3) 编译错误逐个修复：**

```
:make                -- 编译，错误进入 quickfix
:copen               -- 打开 quickfix，预览每个错误位置
o                    -- 跳到错误处修复
```

---

## 快速参考卡片

```
╔═══════════════════════════════════════════════════════╗
║  SURROUND                                             ║
║  ys{motion}{char}  添加包围    ysiw" → "word"        ║
║  cs{old}{new}      修改包围    cs"'  → 'word'        ║
║  ds{char}          删除包围    ds"   → word           ║
║  gs{char}          Visual包围  gs)   → (selection)    ║
╠═══════════════════════════════════════════════════════╣
║  SPIDER                                               ║
║  w/e/b  识别 camelCase 和 snake_case 子词边界         ║
║  W/E/B  仍然是原始 WORD 移动（不受影响）              ║
╠═══════════════════════════════════════════════════════╣
║  TELESCOPE-UNDO                                       ║
║  <leader>su  打开 undo 历史   <CR> 恢复到该状态      ║
╠═══════════════════════════════════════════════════════╣
║  BQF (quickfix 窗口内)                                ║
║  p 预览开关  <C-f> 模糊过滤  <Tab> 标记  zn 只留标记 ║
╚═══════════════════════════════════════════════════════╝
```
