# Recall batteries

这些 pattern 是高 recall probe，不是泄漏定义。每个 hit 都需要语义判断；pattern 会误报，也一定会漏报，因此还必须无关键词阅读 scope 中 prose 最密集的部分。

## Invocation

- 使用 `--hidden --glob '!.git/**'` 搜索 `.agents/` 等隐藏目录。
- inclusion glob 在前，exclusion glob 在后，避免后续 include 重新纳入排除目录。
- 排除第三方、vendored 内容、fixture、snapshot、录制输出、冻结历史和本 Skill 目录。
- natural-language pattern 使用 `-i`；task code 等大小写敏感 pattern 不使用 `-i`。
- zero hit 只说明 pattern 没命中；先用 known-positive string 验证 pattern 自身。
- 语义审计发现这里未列出、且能泛化为搜索特征的 heuristic candidate 时，将最窄的 high-recall probe 补到对应语言区；用 known-positive 验证它，并把稳定的 false-positive boundary 补到下文。不要为单个 passage 添加一次性 pattern。

## English probes

```sh
rg -n --hidden '\(decision \d|\(audit [A-Z]\d|design §|plan §|spec §|\bP-I\b|\bW\d\b|\bT\d\b' <scope> <exclusions>
rg -n --hidden -i 'this PR|this branch|this stack|later PR|previous commit|this commit' <scope> <exclusions>
rg -n --hidden -i 'used to |no longer|previously|the old |was renamed|was moved' <scope> <exclusions>
rg -n --hidden -i '\bv1\b|this cut|\bcut \d|\btoday\b|\bfor now\b|roadmap' <scope> <exclusions>
rg -n --hidden -i 'rejected in review|review round|reviewer|as of v\d' <scope> <exclusions>
rg -n --hidden -i 'probably |should be enough|should suffice|it simply|is safe —|is safe --' <scope> <exclusions>
rg -n --hidden '§\d' <scope> <exclusions>
```

## Chinese probes

```sh
rg -n --hidden '设计稿|评审|上一?轮|旧版|老的|不再|以前|本版|遗留|私有|[A-Z]方案|任务[一二三四五六七八九十0-9]+' <scope> <exclusions>
rg -n --hidden '(^|[^a-zA-Z])端([^a-zA-Z]|$)' --glob '*.md' <scope> <exclusions>
```

## Frequent false positives

- `the key used to sign requests` 中 `used to` 表示用途，不是过去状态。
- `old connection` 与 `new connection` 可能是同时存在的 runtime object。
- PR 模板和贡献流程文档可以讨论 PR；问题是产品文档采用某个 PR 的视角。
- `/v1/chat` 和 wire-format identifier 不是草稿版本 stamp。
- committed document 或外部标准的 section 可以解析。
- ADR 的 `Alternatives considered` 可以写 `rejected`；它记录 decision，不是 review choreography。
- fixture、snapshot、录制输出中的 `today` 或自然语言保留原始声音。
