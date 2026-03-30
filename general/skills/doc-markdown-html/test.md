# 渲染测试文档

本文件覆盖所有 Markdown 格式与渲染特性，用于回归测试。

## Markdown 格式

### 行内格式

普通文本，**加粗**，*斜体*，***加粗斜体***，~~删除线~~，`行内代码`，[超链接](https://example.com)。

### 标题层级

#### H4 标题
##### H5 标题
###### H6 标题

### 引用块

> 单层引用：foo bar baz。
>
> > 嵌套引用：qux quux。

### 列表

无序列表：

- foo
- bar
  - baz（嵌套）
  - qux
- quux

有序列表：

1. 第一步
2. 第二步
   1. 子步骤 a
   2. 子步骤 b
3. 第三步

### 表格

| 名称   | 类型     | 默认值  | 说明           |
| ------ | -------- | :-----: | -------------- |
| foo    | `string` | `"bar"` | 示例字段       |
| baz    | `number` | `42`    | 另一个示例字段 |
| qux    | `bool`   | `true`  | 布尔标志       |

### 定义列表

`foo`
:   Foo 的定义，作为占位示例。

`bar`
:   Bar 的定义，与 foo 相对。

### 脚注

Foo bar baz[^1]，qux quux corge[^2]。

[^1]: 第一条脚注说明。
[^2]: 第二条脚注说明。

### 分隔线

---

本文件专门测试 Victor Mono italic 效果：**关键字**、**字符串**、**注释** 均使用手写斜体字形。

## Python

```python
# 这是一行注释 — 应显示手写斜体
# Single line comment, should be handwritten italic
"""模块级文档字符串，也是斜体"""
"""Doc comment string, should be handwritten italic"""

from __future__ import annotations
import asyncio
from typing import Optional

GREETING = "Hello, world!"           # 字符串 + 注释
TEMPLATE  = f"Count: {1 + 1}"       # f-string 插值


class Greeter:
    """向用户发送问候的类。"""

    def __init__(self, name: str) -> None:
        self.name = name             # 实例变量

    async def greet(self, times: int = 1) -> Optional[str]:
        """异步返回问候语，重复 times 次。"""
        if times <= 0:
            return None
        await asyncio.sleep(0)
        return "\n".join(f"Hi, {self.name}!" for _ in range(times))


def main() -> None:
    greeter = Greeter("世界")
    result = asyncio.run(greeter.greet(3))
    assert result is not None
    print(result)


if __name__ == "__main__":
    main()
```

## Rust

```rust
// 单行注释 — 斜体手写
// Single line comment, should be italic
/// 文档注释，也应显示为斜体
/// Doc comment, shuold be italic

use std::fmt;

const MAX_RETRIES: u32 = 5;

/// 表示一次 HTTP 请求的结果。
#[derive(Debug)]
pub enum Response<T> {
    Ok(T),
    Err(String),
}

impl<T: fmt::Display> fmt::Display for Response<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok(val) => write!(f, "OK: {val}"),
            Self::Err(msg) => write!(f, "Error: {msg}"),
        }
    }
}

fn fetch(url: &str, retries: u32) -> Response<String> {
    if retries == 0 {
        return Response::Err(format!("Gave up after {MAX_RETRIES} retries"));
    }
    // 模拟成功返回
    Response::Ok(format!("body from {url}"))
}

fn main() {
    let resp = fetch("https://example.com", MAX_RETRIES);
    println!("{resp}");
}
```

## TypeScript

```typescript
// 配置接口
interface Config {
  host: string;
  port: number;
  debug?: boolean; // 可选字段
}

const DEFAULT: Config = {
  host: "localhost",
  port: 8080,
  debug: false,
};

/**
 * 将配置序列化为连接字符串。
 * @param cfg - 目标配置
 * @returns 形如 `host:port` 的字符串
 */
function toConnectionString(cfg: Config): string {
  const { host, port } = cfg;
  return `${host}:${port}`;
}

async function connect(cfg: Config = DEFAULT): Promise<void> {
  const addr = toConnectionString(cfg);
  if (cfg.debug) {
    console.log(`Connecting to ${addr}…`);
  }
  await Promise.resolve(); // 模拟异步
}

export { connect, type Config };
```

## Bash

```bash
#!/usr/bin/env bash
# 安装脚本 — 注释斜体
# italic

set -euo pipefail

TARGET="${HOME}/.local/bin"
VERSION="1.2.3"

# 检测操作系统
detect_os() {
  case "$(uname -s)" in
    Darwin) echo "macos" ;;
    Linux)  echo "linux" ;;
    *)      echo "unknown" ;;
  esac
}

OS=$(detect_os)
echo "Running on: ${OS}"

if [[ "$OS" == "unknown" ]]; then
  echo "Unsupported OS" >&2
  exit 1
fi

mkdir -p "$TARGET"
curl -fsSL "https://example.com/releases/${VERSION}/tool-${OS}" \
  -o "${TARGET}/tool"
chmod +x "${TARGET}/tool"
echo "Installed to ${TARGET}/tool"
```

## JSON

```json
{
  "name": "doc-markdown-html",
  "version": "2.0.0",
  "description": "Markdown to HTML renderer with syntax highlighting, string should be italic",
  "features": {
    "theme": "Tokyo Night",
    "font": "Victor Mono",
    "italic": ["keywords", "strings", "comments"]
  },
  "numbers": [1, 3.14, -42, 1e10],
  "flags": { "dark_mode": true, "toc": true, "copy_button": true }
}
```

## 长内容（测试 TOC 滚动高亮）

### 小节 A

Lorem ipsum dolor sit amet，连续段落用于撑开页面高度，以便测试滚动时目录条目的联动高亮效果。

Lorem ipsum dolor sit amet，连续段落用于撑开页面高度，以便测试滚动时目录条目的联动高亮效果。

### 小节 B

Lorem ipsum dolor sit amet，连续段落用于撑开页面高度，以便测试滚动时目录条目的联动高亮效果。

Lorem ipsum dolor sit amet，连续段落用于撑开页面高度，以便测试滚动时目录条目的联动高亮效果。

### 小节 C

Lorem ipsum dolor sit amet，连续段落用于撑开页面高度，以便测试滚动时目录条目的联动高亮效果。

Lorem ipsum dolor sit amet，连续段落用于撑开页面高度，以便测试滚动时目录条目的联动高亮效果。
