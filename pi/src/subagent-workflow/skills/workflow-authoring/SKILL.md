---
name: workflow-authoring
description: Author scripts for the pi-subagent-workflow runtime. This skill explains determinism, replay, concurrency, structured output, and worktree isolation; it does not execute or orchestrate workflows.
---

# Authoring workflow scripts

This skill is an authoring reference. The `pi-subagent-workflow` extension
executes scripts and owns scheduling, concurrency, approval, journaling, and
resume behavior.

A workflow is a JavaScript module executed deterministically so completed work
replays on resume. The script starts with a literal `meta` header and uses the
injected globals; imports are unavailable.

```js
export const meta = { name: 'audit-routes', description: 'Audit routes', phases: [{ title: 'Discover' }, { title: 'Audit' }] }
const result = await agent('List route files', { model: 'low', schema: { type: 'object', properties: { files: { type: 'array', items: { type: 'string' } } }, required: ['files'], additionalProperties: false } })
const files = result?.files.filter(Boolean) ?? []
phase('Audit')
return parallel(files.map(file => () => agent('Audit ' + file, { model: 'mid' })))
```

`meta.name` must be kebab-case; `description` is required; `phases` is
optional. The header must be a literal - no computed values.

## Globals

- `agent(prompt, opts)` - run one subagent. Returns the schema-validated
  value when `schema` is set, otherwise the child's final text. On child
  failure it resolves to `null` (never throws) - guard results before
  dereferencing. With `isolation: 'worktree'` it returns
  `{ value, patch, changed }`; the patch is never applied automatically.
- `parallel(thunks, options?)` - explicit barrier over `() => Promise`
  thunks; result order is preserved. A thunk that throws rejects the barrier
  after launched branch work settles.
- `pipeline(items, options?, ...stages)` - runs each item through the stages
  concurrently. Stages are called as `stage(item, previous)`: the original
  item first, the prior stage's return second (`undefined` in stage one), so
  a later stage that needs the prior result must declare both parameters.
- `phase(title)` - sets the phase for subsequently created agents. Root-idle
  only; inside branches use `opts.phase` instead.
- `log(message)` - progress line surfaced to the user. `console.log(...values)`
  is an alias that stringifies and joins its arguments before forwarding.
- `args` - the deep-frozen deterministic input passed at launch.

`options` on the helpers supports `{ concurrency: positiveInteger }`. Omit it:
the process-wide semaphore already paces all spawns. Pass it only when a
branch group should intentionally run below the global limit.

## agent() options

`model`, `thinkingLevel`, `tools`, `excludeTools`, `schema`, `cwd`,
`isolation`, `label`, `phase`.

- `model` is required and must be `"high"`, `"mid"`, or `"low"`. Direct
  provider/model-id values are rejected. The parent system prompt shows the
  current tier-to-model mappings; these IDs are informational, not arguments.
- Choose `low` for targeted lookup, extraction, formatting, and mechanical
  edits; `mid` for ordinary implementation, bounded debugging, tests, and
  focused review; `high` for complex root-cause analysis, cross-module design,
  substantial uncertainty, or costly mistakes. Use the lowest adequate tier
  and respect an explicit user choice. File count and prompt length alone do
  not determine difficulty. `thinkingLevel` is independent of the model tier.
- Tiers are configured by the user through `/model-tiers` or the `model-tier`
  field in `~/.pi/agent/config.json`. Unconfigured or unavailable tiers fail;
  ask the user to fix the binding instead of changing it or substituting a tier.
  Each launch/resume snapshots the mappings so in-flight calls keep their models.
- An explicit `tools` list is a capability contract: a requested name that
  does not resolve for the child fails that call with a missing-tool
  diagnostic instead of silently running without it.
- Every prompt must be self-contained. The child receives neither the parent
  conversation nor workflow variables - interpolate what it needs.
- Omit `tools`/`excludeTools` unless deliberately constraining a child.
- Compose specs per task. Recurring shapes belong in skills or saved
  workflows, never inline personas.

## Concurrency model

Each root or branch scope may have only one active `agent()` call or branch
group; use `parallel`/`pipeline` for concurrency, never raw Promise
combinators. Host calls start concurrently up to the helper's bound, while
agent-dependent continuations resume in accepted call order, so provider
timing cannot change shared state. Unawaited calls drain before workflow or
branch completion. Helper inputs must be dense ordinary arrays of data
elements; accessors, sparse arrays, and custom prototypes are rejected before
any branch starts.

## Determinism rules

Unavailable inside the VM: `Date.now`, `Date.parse`, local-time Date methods,
locale-sensitive APIs, `Intl`, `Math.random`, `Promise.all`/`allSettled`/
`race`/`any`, proxies, nested realms, error stack frames, GC observers,
timers, `process`, `require`, `fetch`, shared-memory intrinsics, and dynamic
`import`. Construct Dates only as `new Date(epochMs)` from one primitive
finite number (a `Date.UTC` value is fine); strings, Date objects, and
component arguments are rejected. Anything that varies between runs -
timestamps, seeds, randomness - must enter through `args`.

## Replay and resume

Every completed `agent()` call is journaled under its stable causal call
identity, a hash of its resolved prompt/options/phase, and a fingerprint of
its resolved execution environment (provider, model, thinking level, cwd,
statically declared extension tool names for the child's cwd). Resume
(`resumeRunId` plus exactly one of `script`/`scriptPath`) re-executes from the
top: matching entries replay instantly; an edited call and its causal tail run
live while completed sibling branches are preserved. Failed calls are never
journaled, so resume retries them.

Changing a tier binding changes the resolved model fingerprint. A completed
call whose environment changed fails the resume closed, naming
the drifted fields and the childId. Recovery is one mechanism: resume with
`rerunChildIds: ["<childId from the error>"]` to authorize re-running exactly
that entry and its causal tail once, or run the workflow fresh. Repository
contents are deliberately outside the fingerprint; when such state must
participate in replay identity, interpolate it (e.g. a revision id) into the
prompt - a changed prompt already reruns.

On resume, omit `args` to reuse the persisted value; passing `args` overrides
it (and reruns calls whose payloads change). A `generation.pending` marker
left by a crash quarantines the run directory: run the workflow fresh and
delete the quarantined directory rather than removing the marker.

## Worktree isolation

`isolation: 'worktree'` gives the child its own checkout at the same
repo-relative cwd. Changes come back as `patch` for explicit review - never
auto-applied or auto-committed to the source branch. A child may commit
inside its detached worktree; those commits are captured in the returned
patch and never touch the source branch. Oversized patches fail collection
and retain the worktree rather than silently degrading.
