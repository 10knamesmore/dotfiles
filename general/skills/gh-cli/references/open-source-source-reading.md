# 子工作流：阅读某个开源项目源码

## 目标

当用户想理解某个 GitHub 开源项目的结构、入口、关键模块、实现方式或某个功能的代码路径时，使用这个工作流。

## 工作流

1. 先检查 `gh` 是否可用。
   优先运行 `gh auth status`。
   源码阅读常常依赖仓库元数据和代码搜索，不要跳过这一步。

2. 先建立项目上下文。
   先看仓库主页、README、目录结构、发布信息和默认分支。
   先回答“这个项目做什么、主要技术栈是什么、核心入口在哪里”。

3. 明确阅读目标。
   区分用户是要：
   整体架构、某个功能实现、某段调用链、扩展点、配置加载方式，还是 bug 相关路径。

4. 先读结构，再读实现。
   优先看 README、文档目录、主入口文件、模块划分和测试目录。
   不要一上来全局搜索所有细节。

5. 沿调用链阅读关键文件。
   从入口函数、命令注册、HTTP 路由、插件加载点、配置初始化或主循环开始。
   再追到具体模块和数据结构。

6. 用搜索验证理解。
   用 `gh search code` 或本地全文搜索验证符号、接口、配置项和关键路径。
   找到测试或示例时，一并用来佐证理解。

7. 输出时强调“结论 + 证据”。
   说明功能在哪里实现、调用链怎么走、关键文件有哪些、哪些点仍需继续确认。

## 常用命令

- 查看仓库：`gh repo view OWNER/REPO`
- 结构化查看仓库元数据：`gh repo view OWNER/REPO --json name,description,defaultBranchRef,licenseInfo,stargazerCount,updatedAt`
- 搜索代码：`gh search code "symbol repo:OWNER/REPO"`
- 搜索配置项：`gh search code "config_name repo:OWNER/REPO"`
- 搜索入口函数：`gh search code "main( repo:OWNER/REPO"`
- 搜索根命令或入口构造函数：`gh search code "NewCmdRoot repo:OWNER/REPO"`
- 需要深读时克隆到本地：`gh repo clone OWNER/REPO`

## 阅读顺序建议

- 先看 README、docs、examples
- 再看入口文件和初始化逻辑
- 再看核心模块、数据结构、接口定义
- 最后看测试、fixture、样例配置和边缘逻辑

对于 CLI、服务端或插件类项目，优先从这些符号或目录开始搜：

- `func main`
- `NewCmdRoot`
- `cmd/`
- `internal/`
- `pkg/`

## 输出要求

- 先给高层结构，再给关键代码路径
- 文件路径尽量具体
- 如果结论来自推断，要明确标注是推断
- 如果只看了 GitHub 网页和搜索结果，明确说明尚未本地运行或验证
