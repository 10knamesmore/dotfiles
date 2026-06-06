---
name: gen-test-plan
description: 编写测试计划时使用。适用于功能测试、集成测试、回归测试、验收测试等场景。
---

# Doc Test Plan

## 工作流

1. 确认测试对象（项目/模块/功能）和输出文件名。
2. 基于下方章节骨架和 fixture 规范，撰写完整的测试计划 Markdown 文件。
3. 运行渲染脚本生成 HTML（默认会删除源 .md 文件，仅保留 HTML）。
4. 返回 HTML 文件的绝对路径，并简要说明核心章节。
   - 返回路径格式如: `file:///User/foo/bar.html`
   - 若用户要求保留 md 文件，使用 `--keep-md` 参数，同时返回 md 路径

## 章节骨架

测试计划文档必须包含以下章节（按顺序）：

### Frontmatter

```yaml
---
project: 项目名称
version: 版本号
author: 作者
date: "YYYY-MM-DD"
status: Draft | In Review | Approved
module: 被测模块
test-type:
  - 单元测试
  - 集成测试
  - E2E测试
---
```

### 正文章节

1. **概述** — 测试目的、测试范围（明确包含和排除的内容）
2. **测试策略** — 测试类型说明、测试层级划分、优先级定义（P0/P1/P2/P3）
3. **测试环境** — 硬件/软件/网络要求、第三方服务依赖、数据库版本等
4. **Fixtures（前置条件与测试数据）** — 见下方 Fixture 规范
5. **测试用例** — 用例表格，见下方格式
6. **进入/退出标准** — 开始测试和结束测试的条件
7. **风险与缓解措施** — 已知风险、影响评估、缓解方案
8. **时间安排与资源分配** — 里程碑、负责人、工时估算

## Fixture 规范

测试计划中必须包含 **Fixtures** 章节，用代码块提供测试所需的前置数据。每个 fixture 需要：

- 用带语言标识的代码块包裹
- 在代码块前说明用途和关联的测试用例 ID
- 标注 fixture 的使用方式（直接导入 / API mock / 数据库种子等）

常见 fixture 类型：

- **JSON fixture**: 请求体、响应体、配置文件、种子数据
- **SQL 脚本**: 数据库初始化、测试数据插入、清理脚本
- **环境变量 / 配置模板**: `.env` 模板、配置文件
- **API Mock 定义**: 请求/响应映射

示例：

````markdown
### Fixtures

#### 用户数据 fixture

用于 TC-001、TC-002、TC-003，通过数据库种子脚本导入。

```json title="fixtures/users.json"
[
  {
    "id": 1,
    "username": "test_admin",
    "role": "admin",
    "email": "admin@test.com"
  },
  {
    "id": 2,
    "username": "test_user",
    "role": "member",
    "email": "user@test.com"
  }
]
```

#### 数据库初始化

用于所有测试用例，在测试套件启动前执行。

```sql title="fixtures/init.sql"
INSERT INTO users (id, username, role, email)
VALUES (1, 'test_admin', 'admin', 'admin@test.com'),
       (2, 'test_user', 'member', 'user@test.com');
```
````

## 测试用例表格格式

| ID | 模块 | 描述 | 前置条件 / Fixture | 步骤 | 预期结果 | 优先级 |
|---|---|---|---|---|---|---|
| TC-001 | 模块名 | 用例描述 | 引用 fixture 名称 | 编号步骤 | 预期行为 | P0-P3 |

优先级定义：
- **P0**: 阻断性，必须通过才能继续
- **P1**: 核心功能，高优先级
- **P2**: 重要但非关键
- **P3**: 边缘场景、低优先级

## 渲染命令

```bash
uv run ../doc-markdown-html/scripts/render_markdown_html.py <input.md> --output <output.html>
```

当你发现在沙箱环境下无法运行时，你应当向用户要求提权运行 uv，而不是尝试用其他方式运行脚本。

参数：

| 参数 | 简写 | 说明 | 默认值 |
|---|---|---|---|
| `<input>` | — | 输入 Markdown 文件路径（必填） | — |
| `--output` | `-o` | 输出 HTML 文件路径 | 同输入路径，扩展名改为 `.html` |
| `--template` | `-t` | HTML 模板文件路径 | `assets/doc-template.html` |
| `--title` | — | 强制指定页面标题 | 取第一个 `# 标题`，无则用文件名 |
| `--toc-min-level` | — | 目录包含的最小标题级别 | `2` |
| `--toc-max-level` | — | 目录包含的最大标题级别 | `3` |
| `--open` | `-O` | 渲染完成后在浏览器中打开 | 否 |
| `--keep-md` | — | 保留源 Markdown 文件（默认渲染后删除） | 否 |

## 参考文档

**重要：在编写任何测试计划之前，必须先用 Read 工具阅读同目录下的 `reference.md`。** 这是一份完整的标准测试计划示例，定义了章节结构、Fixture 写法、测试用例格式等所有细节。你的输出必须在结构和风格上与该参考文档保持一致。

## 规则

- 始终以 Markdown 作为事实来源。
- 除非用户明确要求，否则始终生成 HTML。
- 支持解析 Markdown 文件开头的 YAML frontmatter。
- 若用户已提供现成 Markdown 文件路径，则不允许修改原有的 Markdown 文件。
- 不允许修改 `../doc-markdown-html/assets/doc-template.html`。
- 测试计划必须包含 Fixtures 章节，至少提供一个 fixture 代码块。
- 测试用例表格必须包含优先级列。
- 每个 fixture 代码块前必须说明用途和关联的测试用例 ID。
- frontmatter 中的日期值必须加引号（如 `date: "2026-04-01"`），否则 YAML 会将其解析为 `datetime.date` 对象导致 JSON 序列化失败。
