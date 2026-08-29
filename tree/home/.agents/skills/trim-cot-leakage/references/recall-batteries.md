# Recall batteries

这些 pattern 是高 recall probe，不是泄漏定义。每个 hit 都需要语义判断；pattern 会误报，也一定会漏报，因此还必须无关键词阅读 scope 中 prose 最密集的部分。

## Invocation

- 一键版本：[`../scripts/recall-batteries.sh`](../scripts/recall-batteries.sh) `[scope] [额外排除 glob...]`，内置排除 `.git` 和本 Skill 目录，逐条 probe 带标签输出。
- probe 的唯一真相源是 [`recall-batteries.tsv`](./recall-batteries.tsv)：每行 `label<TAB>flags<TAB>pattern<TAB>glob`（glob 可空，flags 含 `i` 表示 `-i`），`#` 开头为注释。自定义场景直接从 TSV 取 pattern 拼自己的 rg 命令。
- 使用 `--hidden --glob '!.git/**'` 搜索 `.agents/` 等隐藏目录。
- inclusion glob 在前，exclusion glob 在后，避免后续 include 重新纳入排除目录；脚本已按此顺序组装。
- 排除第三方、vendored 内容、fixture、snapshot、录制输出、冻结历史和本 Skill 目录。
- natural-language pattern 使用 `-i`；task code 等大小写敏感 pattern 不使用 `-i`。
- zero hit 只说明 pattern 没命中；先用 known-positive string 验证 pattern 自身。
- 语义审计发现 TSV 未列出、且能泛化为搜索特征的 heuristic candidate 时，将最窄的 high-recall probe 作为一行补进 TSV；用 known-positive 验证它，并把稳定的 false-positive boundary 补到下文。不要为单个 passage 添加一次性 pattern。

## Frequent false positives

- `the key used to sign requests` 中 `used to` 表示用途，不是过去状态。
- `the currently focused window`、`current connection` 等描述 runtime state，不是过期快照。
- changelog 与 release notes 中的版本号和日期是这些 surface 的职责。
- `old connection` 与 `new connection` 可能是同时存在的 runtime object。
- PR 模板和贡献流程文档可以讨论 PR；问题是产品文档采用某个 PR 的视角。
- `/v1/chat` 和 wire-format identifier 不是草稿版本 stamp。
- committed document 或外部标准的 section 可以解析。
- ADR 的 `Alternatives considered` 可以写 `rejected`；它记录 decision，不是 review choreography。
- fixture、snapshot、录制输出中的 `today` 或自然语言保留原始声音。
