# AST grep

`ast_grep` 使用 PATH 中的 `ast-grep`，提供按语法结构搜索和替换代码的单一工具。安装由 `dots.lua` 的 Cargo 清单管理；扩展通过 `pi-extensions` 分发。

## 调用

```typescript
// 搜索；$USER 和 $OPTIONS 捕获语法节点。
ast_grep({
  paths: ["crates/"],
  language: "rust",
  pattern: "old_api($USER, $OPTIONS)",
});

// 预览 diff，不写入。
ast_grep({
  paths: ["crates/"],
  language: "rust",
  pattern: "old_api($USER, $OPTIONS)",
  replacement: "new_api($OPTIONS, $USER)",
});

// 应用同一规则。
ast_grep({
  paths: ["crates/"],
  language: "rust",
  pattern: "old_api($USER, $OPTIONS)",
  replacement: "new_api($OPTIONS, $USER)",
  preview: false,
});
```

`pattern` 和 `rule` 必须且只能提供一个。复杂查询直接传 [ast-grep Rule 对象](https://ast-grep.github.io/reference/rule.html)，不是带有 `id`、`language` 和 `fix` 的完整配置：

```typescript
ast_grep({
  paths: ["src/"],
  language: "rust",
  rule: {
    pattern: "$VALUE.unwrap()",
    inside: {
      kind: "function_item",
      stopBy: "end",
    },
  },
  globs: ["!**/generated/**"],
});
```

`$$$ARGS` 捕获多个节点；`replacement: ""` 删除匹配节点。路径可相对当前工作目录或为绝对路径。文件发现沿用 ast-grep 的 ignore 行为，`globs` 沿用其 `--globs` 语义。

## 结果与写入

- 搜索返回匹配代码、文件、范围和捕获变量。结构化结果的行、列和 UTF-8 字节偏移从 0 开始，结束位置不包含在范围内。
- 替换默认返回 ast-grep 原生预览 diff。`preview: false` 重新扫描当前文件并写入；预览不是冻结的待提交补丁。
- 写入先发现候选文件，再持有 Pi 文件修改队列锁重新匹配并调用 ast-grep。应用结果包含文件写入前后的实际 diff；多个文件之间不提供事务或回滚。
- `matchCount` 是匹配节点数，不保证等于实际修改数，例如嵌套匹配或替换前后相同时。
- 模型文本超过 2000 行或 50KB 时截断，并提供完整 JSON 结果的临时路径。程序调用通过 `structuredContent` 获取完整结果。

语法查询不解析符号身份或推导类型。跨文件符号重命名应使用语言服务器；任意文本编辑仍使用 `apply_patch`。
