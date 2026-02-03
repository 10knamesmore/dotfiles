---
name: po-fixture-indexer
description: Generate or update a semantic index for Playwright `fixtures/` and `pages/` by scanning source files, inferring business semantics, and writing `index/fixtures/*.yml`, `index/pages/*.yml`, plus `INDEX.md` overviews. Use when asked to rescan fixtures/pages, refresh capability indexes, list available UI actions, or update indexes after new fixture/page object methods are added.
---

# Po Fixture Indexer

## Overview

Scan `fixtures/` and `pages/`, extract exported objects/classes/methods, infer business semantics, and generate a machine-parseable semantic index under `index/fixtures` and `index/pages` with Chinese business descriptions.

## Workflow

### 1) Scan inventory (raw signal)

- Read all `*.ts` in `fixtures/` and `pages/` (exclude `*.d.ts`).
- Extract:
  - Class names / exported objects
  - Method names (especially page-object actions)
  - Comments (leading `//` or `/** */`)
  - File paths
- Recommended helper script (heuristic scan):

```bash
python3 /Users/xzmini43/.codex/skills/po-fixture-indexer/scripts/scan_ts_inventory.py \
  --root <repo-root> \
  --out /tmp/po-fixture-inventory.json
```

- If the script misses semantics, open the file and read the class/fixture body directly.

### 2) Infer business semantics

- Decide the **business meaning** of each fixture/page object, not technical details.
- Produce a semantic name:
  - lower-case, kebab-case
  - business-oriented
  - no technical hints (no `page`, `fixture`, `helper`)
- Examples:
  - `LoginPage` -> `login-flow`
  - `AccountFixture` -> `account-bootstrap`
  - `StrategyControlPage` -> `strategy-control`

### 3) Generate YAML semantic index files

For each fixture/page object (or exported fixture), write one YAML file:

- Fixtures: `index/fixtures/<semantic-name>.yml`
- Pages: `index/pages/<semantic-name>.yml`

YAML schema (keep stable and machine-parseable):

```yaml
name: login-flow
type: fixture # fixture | page
source: fixtures/login.fixture.ts

description: >
  负责初始化登录态，
  提供统一的登录入口，
  供所有测试用例复用。

responsibilities:
  - 创建登录会话
  - 管理 token 生命周期
  - 提供已登录页面上下文

business_methods:
  - name: loginAsAdmin
    purpose: 使用管理员账号登录
    side_effects:
      - 设置 cookie
      - 跳转 dashboard

  - name: loginAsUser
    purpose: 使用普通用户登录
    side_effects:
      - 初始化用户环境

dependencies:
  - auth-service
  - dashboard-page

used_by:
  - split-order tests
  - strategy tests
```

Rules:
- `description` / `responsibilities` / `purpose` must be **business semantics in Chinese**.
- Avoid implementation details (e.g., selectors, API endpoints, internal class names).
- If unknown, keep arrays empty (`[]`) or keep entries brief rather than guessing technicals.

### 4) Generate INDEX.md for each directory

Create:
- `index/fixtures/INDEX.md`
- `index/pages/INDEX.md`

Format:

```md
# Fixtures Index

本目录包含项目所有 fixture 的语义索引。

## login-flow
登录流程管理
→ fixtures/login.fixture.ts

## account-bootstrap
账户初始化
→ fixtures/account.fixture.ts
```

Pages index uses the same layout with `# Pages Index`.

### 5) Overwrite existing index

- Treat `index/fixtures` and `index/pages` as **generated**.
- Rebuild them completely on each run.
- Do not edit `playwright-report/` or `test-results/`.

## Heuristics for semantic classification

Use these signals:
- Class/fixture names (`Login`, `Account`, `Order`, `Strategy`, `Nav`)
- Method names (`loginAsAdmin`, `createOrder`, `openStrategy`)
- Leading comments/docstrings
- Test usage in `tests/` (if needed, `rg "<ClassName>" tests`)

If multiple business meanings exist, split into multiple semantic entries (one per exported object/class).

## Resources

### scripts/

- `scan_ts_inventory.py`: Heuristic inventory scanner for `.ts` files. Use to bootstrap extraction of exports/classes/methods.

