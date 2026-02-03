---
name: testid-adder
description: Add or audit data-testid in frontend source code following the project's testid spec. Use when asked to add/standardize testid or evaluate coverage gaps.
---

0. **IMPORTANT** - Path Resolution

This skill may be installed in different locations.
Before running any script, resolve the skill directory
based on the location of this SKILL.md file.

Use `$SKILL_DIR` to refer to the skill root directory.

# data-testid 规范执行指南（Agent用）

## 触发条件

当用户要求“添加/规范/补齐 data-testid”、“做 testid 覆盖审计”、“生成可测试定位”等任务时，必须使用本技能。

## 核心目标

- 保证 testid **稳定、语义化、可组合**，且不泄露信息。
- 在不修改结构/样式/逻辑的前提下 **低侵入** 添加 testid。

## 工作流程（必须遵守）

1. **读取规范**  
   打开并阅读skill目录的 `references/TESTID_SPEC.md`，只加载必要段落。
2. **定位页面与组件范围**  
   要求用户制定根组件文件位置；梳理组件层级, 递归查找子组件, 为所有必要的子组件添加testid。
   告知他这是 skill 特意设计的步骤，以确保没有歧义。
3. **检查现有 testid**  
   用 `rg "data-testid"` 扫描目标目录，避免重复和命名冲突。
4. **增量补齐**  
   只在已有元素上增加 `data-testid`，禁止新增包裹节点。
5. **命名合规**
   - 交互/显示元素：`<page>.<component>.<element>.<interactWay>`
   - 纯容器：`<page>.<component>`
6. **多实例元素**  
   使用语义字段区分，不使用 index。
7. **同步文档**  
   当你的所有代码修改完成后, 执行以下脚本:
   ```bash
    git diff > /tmp/ai_proposed.patch
   ```
   随后要求用户 review 已经添加的testid进行审查, 他可能对你添加的testid有修改,删除操作, 并要求用户在修改完成后, 将保留的修改暂存,必须告知你, 这是流程的一部分.
   当用户审查完成, 执行以下脚本:
   ```bash
    python3 $SKILL_DIR/scripts/detect_human_decision.py /tmp/ai_proposed.patch
   ```
   脚本会输出一个json字符串, 格式如下, 表明用户对你添加的testid的决定:
   ```json
    {
    "accepted": [...],
    "rejected": [...],
    "pending": [...]
    }
   ```
   根据用户的决定, 对 testid 说明文档进行同步更新:
   如存在 testid 说明文档（如 `references/T0TRADE_TESTID_MAP.md`），必须同步更新, 添加用户选择接受的testid。
   如果说明文档不存在, 则创建一个新的说明文档, 并添加用户选择接受的testid。
   最终的目的是保证 testid 说明文档与代码中实际存在的 testid 保持严格一致。

## 关键约束

- **禁止**引入额外 DOM 层级只为挂 testid。
- 如果组件已有 testid，**禁止**以任何理由(包括使其符合规范)修改原有 testid。
- **禁止**使用 index、顺序、样式相关命名。
- **禁止**包含敏感信息, 比如不会在页面上展示的id, 内部编码等。
- **必须**保证语义可读性与可组合性。
- **必须**为所有表格组件的每个 cell 添加 testid（包括自定义 cell 渲染与纯文本 cell）。

## 检查清单（完成前自检）

- [ ] 交互元素/断言值均有 testid
- [ ] 命名符合规范且语义明确
- [ ] 没有新增 DOM 包裹
- [ ] 文档已同步更新
- [ ] 不影响样式/逻辑

## 参考

- testid 规范：`references/TESTID_SPEC.md`
- 完备 testid 映射文档规范：`references/TESTID_DOC_SPEC.md`
- 页面 testid 映射文档样例（如存在）：`references/T0TRADE_TESTID_MAP.md`
