# vendor — 冻结的第三方 zsh 插件

手动 clone、删 `.git` 冻结、不跟上游更新。各插件的 README / 图片 / 测试 / CI 已删，只留运行时文件 + LICENSE，出处与冻结点记于此表。

更新某个插件：重新 clone 对应 repo → checkout 目标 commit → 删 `.git` → 按需精简 → 更新本表冻结点。

| 插件 | 上游 | 冻结点 |
|------|------|--------|
| [fast-syntax-highlighting](https://github.com/zdharma-continuum/fast-syntax-highlighting) | zdharma-continuum | v1.55 · `0b13c5b`（2025-06-04） |
| [fzf-tab](https://github.com/Aloxaf/fzf-tab) | Aloxaf | `a731927`（2024-02-18，master 快照、无 release tag） |
| [zsh-autosuggestions](https://github.com/zsh-users/zsh-autosuggestions) | zsh-users | v0.7.0 · `ae315ded`（2020-04-15） |

> 出处 URL 取自各插件文件内部声明。fzf-tab / zsh-autosuggestions 当初冻结时删了 `.git`，commit 由上游内容指纹比对考古而得（vendor 现存文件与该 commit 逐字节一致）：
> - **zsh-autosuggestions**：文件内 `ZSH_AUTOSUGGEST_VERSION` 写 `0.7.1`，但那是 release 时提前 bump 的内部号；整棵树（主文件 + 8 个 src）逐字节等于 tag **v0.7.0**，故以 v0.7.0 为准。
> - **fzf-tab**：无版本变量，是 master 快照（非 release）；上游此后已领先约 44 个运行时 commit，需要时再考虑升级。
