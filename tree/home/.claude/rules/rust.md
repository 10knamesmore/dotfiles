---
paths:
  - "**/*.rs"
---

# Rust

- 不要写 `/*param_name*/` 这类实参注释——我看 inlay hint，注释是冗余噪音。不透明调用点靠 API 设计消灭（枚举 / 具名方法 / newtype），不靠注释打补丁
- 哈希容器如无特殊理由用 fxhash（`FxHashMap` / `FxHashSet`），不用 std 默认的 SipHash；仅在 key 来自不可信外部输入、需要抗 HashDoS 时才用默认 hasher
- 没有必须的理由, 一律使用typed struct, serde_json::Value, json!()宏, 这种会让契约模糊, 难以排查, 只有极少数情况如`这里就是应当接收任意json, 因为只需要透传` 等等才允许使用json::Value
