# Prose examples

这些例子用于识别原则，不是文本模板。`Balanced` 版本在当前位置保留每个 load-bearing proposition，同时删除无助于使用和维护的说明。

## Preserve every factual clause

**Original:** The coordinator carefully serializes writes per session, flushes buffered events before disposal resolves, and reports backend failures to the caller.

**Over-trimmed:** The coordinator serializes persistence.

**Balanced:** The coordinator serializes writes per session, flushes buffered events before disposal resolves, and reports backend failures to the caller.

per-session scope、disposal ordering 和 failure visibility 是三个独立事实，不能因为 `serializes persistence` 更短就合并丢失。

## Preserve ownership and timing

**Over-trimmed:** Provider work is cancelled during teardown.

**Balanced:** The runtime requests provider cancellation before releasing the child scope; the provider remains responsible for joining its workers before disposal resolves.

actor、ordering、ownership 交接点和 completion guarantee 都是 contract。

## Public API docs include failures

**Over-trimmed:** Returns the initialized realm global.

**Balanced:** Returns the initialized realm global. Throws if initialization has not completed or the realm has already been disposed.

前置状态和错误是调用者可见行为，不是实现细节。

## Keep local behavior while linking rationale

**Over-trimmed:** Disposal is documented in the lifecycle ADR.

**Balanced:** Disposal aborts the run and waits for provider quiescence. See the lifecycle ADR for ownership and race handling.

链接不能替代调用点所需的行为和完成保证。

## Orient complicated code without narrating it

**Over-detailed:** 逐段预告下面每个 class、helper 和 callback 的执行顺序。

**Balanced:** Owns the worker realm and its host bridge. Initialization is single-shot; disposal terminates the worker and rejects later calls. See the worker-isolation ADR for the protocol rationale.

保留模块角色、责任和非显然 lifecycle；让代码表达局部控制流。

## Delete reasoning transcripts

**Over-detailed:** First the loop checks whether the value is absent. If it is absent, the next branch returns early. Otherwise it continues, which is why the final assertion is safe.

**Balanced:** 当代码已表达这些分支时不写注释；如果 early return 保护了非显然不变量，只写该不变量和误改后果。

不要把 reasoning transcript 压缩成更短的 walkthrough。

## Configuration comments explain consequences

**Over-detailed:** This entry loads the filesystem provider, followed by the policy plugin, followed by the read and write tools.

**Balanced:** Load policy before model-facing tools so write calls pass through the read-before-mutation gate.

配置树已展示 inventory；注释只保留顺序的业务或安全后果。

## Generated summaries stand alone

**Over-trimmed:** Approval request and policy service.

**Balanced:** Approval service that applies session policy before answerers and logs every request/outcome pair to the requesting session.

生成器抽取的 fragment 必须独立表达其 surface 所需 contract；其余 lifecycle 细节留在 owner 中。

## Limitations are contracts

**Over-trimmed:** 完全省略 process-lifetime cache。

**Over-detailed:** 罗列没有 caller 或 maintainer 后果的私有 helper cleanup。

**Balanced:** Provider selection is cached for the plugin lifetime; installing or repairing a provider requires reload.

保留影响使用或安全维护的限制，不把 README 写成 backlog inventory。

## Current-state artifacts do not narrate the edit

**Leaked:** This PR replaces the old registry with the new catalog.

**Balanced:** The catalog owns registration and duplicate-name rejection.

当前状态 surface 说明现有 owner 和行为；变更历史留在 commit、changelog、ADR 或 postmortem。

## Active proposals may use future tense

**Incorrect trim:** 把活动 Spec 中尚未实现的 proposal 改写成已经 shipped 的事实。

**Balanced:** 活动 proposal 明确写预期状态和 acceptance criteria；完成后再把 resolution 写成实际结果。

删掉 planning marker 不能把假设升级成事实。
