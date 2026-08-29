# Dots Resource Management

Dots 把仓库声明收敛为本机配置，并只自动改变自己明确拥有的状态。

## Language

**Resource**:
Dots 跨多次 sync 持续拥有并收敛的一项本机状态。
_Avoid_: Effect, output

**Declaration**:
仓库当前要求一个 Resource 达到的状态。Declaration 消失表示该 Resource 不再属于 Desired Set。
_Avoid_: Command, hook

**Authoritative Declaration**:
一次 sync 中唯一有权决定某个 Ownership Surface 最终状态的 Declaration。同一实际对象的同一部分最多有一个 Authoritative Declaration。
_Avoid_: Winner, overlay

**Desired Set**:
一次 sync 从当前仓库和当前机器上下文得到的完整 Resource 集合。
_Avoid_: Current config

**Applied Inventory**:
本机记录的、上一次成功由 dots 拥有的 Resource 集合。它是判断声明删除和安全清理的依据。
_Avoid_: Cache, state

**Observed State**:
sync 开始时在 Resource 所拥有位置读到的真实机器状态。
_Avoid_: Current state

**Ownership Surface**:
一个 Resource 独占管理的最小位置，例如一个路径、文本 marker 区间或 systemd user unit。
_Avoid_: Target

**Retired Resource**:
仍在 Applied Inventory、但已不在 Desired Set 的 Resource。
_Avoid_: Orphan

**Drift**:
Observed State 已偏离 dots 上次成功应用的状态，继续自动更新或删除可能破坏用户改动。
_Avoid_: Collision

**Collision**:
Dots 尚未拥有的 Ownership Surface 已被与 Declaration 不同的状态占用。
_Avoid_: Drift

**Derivation**:
planning 期间根据 Declaration 生成 Resource payload 的可重建过程。Derivation cache 不属于 Ownership Surface，也不随 Resource retirement 自动删除。
_Avoid_: Resource, install
