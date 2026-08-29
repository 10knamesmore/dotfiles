# Leakage examples

这些例子用于识别 governing principle，不是替换模板。例子故意包含泄漏 wording，因此审计时应排除本 Skill 目录。

## Dead citations

**Leaked:** Slash input resolves against the visible catalog `(decision 21)`.

**Fixed with owner:** Slash input resolves against the visible catalog; see the committed input-pipeline ADR.

**Fixed without owner:** The registry rejects duplicate names; names are flat, with no namespacing.

不可解析的 ordinal 被删除，但 ordinal 后面的事实仍需独立成立。

## PR vantage

**Leaked:** This PR adds cursor-based pagination to the session list.

**Fixed:** The session list paginates by cursor.

README 会活得比 PR 久，应该陈述当前机制。

## Change narration

**Leaked:** Colors used to come from undefined widget tokens, so the alias tokens fixed the fallbacks.

**Fixed:** Colors come from the alias tokens; an undefined token renders the fallbacks.

当前 mechanism 和 standing failure behavior 都是事实；bug biography 不属于代码注释或 README。

## Regression as present counterfactual

**Leaked:** This used to double-encode multibyte labels.

**Fixed:** Without the byte-length guard, multibyte labels double-encode.

counterfactual 说明不能删除的 guard，并且不要求读者做 repo archaeology。

## Review choreography

**Leaked:** Rejected in review: caching the resolved spec. We keep per-call resolution.

**Fixed in an ADR:** **Caching the resolved spec.** Rejected because the spec depends on per-call cwd, so request-only caching would serve stale roots.

保留 alternative 与 rationale，删除 reviewer 和 round。

## Reviewer-addressed justification

**Leaked:** The cast is safe; the SDK simply does not declare its optionals strictly enough.

**Fixed:** The SDK constructs this object with every optional populated; the declared type is looser than the runtime guarantee.

写 invariant，而不是回答一场仓库中不存在的争论。

## Control-flow narration

**Leaked:** First normalize the label, then truncate it, then wrap it.

**Fixed:** 当相邻代码已经表达这些步骤时删除整句。

## Test walkthrough

**Leaked:** This test creates a session, sends two messages, waits, then asserts four log entries.

**Fixed:** Two round-trips produce four log entries because the projection deduplicates the shared prefix.

保留 assertion 的非显然原因，不复述测试体。

## Vague sizing

**Leaked:** A 64 KiB buffer should be enough for most cases.

**Fixed:** 64 KiB holds the largest observed frame of 48 KiB with headroom; a larger frame fails in `decode`.

用真实 bound、来源和超限行为代替 hedge。

## Stale snapshot

**Leaked:** Production currently runs 2.3.1; consult the 2.3.1 changelog when debugging live behavior.

**Fixed:** 调试线上行为前先确认部署版本：`deployctl status` 输出当前 commit；各版本变更见 changelog。

长寿命页面记录读取答案的命令，不记录答案；版本号属于 changelog，四个发布之后快照自行变成错误声明。

## Pointer without a symbol

**Leaked:** Protocol versions are defined in the networking layer.

**Fixed:** Protocol versions are defined by `PROTOCOL_VERSION` in `net/constants.py`.

没有任何单一符号拥有该事实时，先修代码边界，不要在文档里复制字段表。

## Legitimate keeps

- `issue #1470 owns the follow-up` 在仓库流程中可以解析。
- `measured: 512 nests take about 0.15s synchronously` 为常量提供 provenance。
- `The old connection drains before the new one accepts` 描述同一时刻的 runtime object。
- `RFC 9110 section 10.1.5` 指向稳定外部标准。
- `Without the guard, the callback can publish after disposal` 是 present-tense regression pin。

## Overcorrection traps

### Obligation becomes endorsement

**Original:** These direct registrations are exceptions pending migration to slots.

**Wrong:** These direct registrations are sanctioned exceptions.

`pending migration` 是 obligation；`sanctioned` 反而认可现状。

### Hypothetical becomes shipped fact

**Original:** A future IPC shell would override `spawn`.

**Wrong:** An IPC shell overrides `spawn`.

**Balanced:** A hypothetical IPC shell would override `spawn`; no such shell is currently shipped.

只删除 future marker 会把设计示例升级成不存在的产品事实。

### True fact is deleted with narration

**Original:** The notice narrates check order; its text is also compiled by `verify-doc-typecheck`.

**Wrong:** 删除整句。

**Balanced:** The notice text is compiled by `verify-doc-typecheck`.

同一句中的 narration 和 load-bearing coupling 必须分开判断。

### Measurement provenance disappears

**Original:** The 4 MiB ceiling is measured; the largest generated module is 3.1 MiB.

**Wrong:** The ceiling is 4 MiB; the largest generated module is 3.1 MiB.

保留 `measured` 和测量条件，否则 observation 会被误读为 definition。
