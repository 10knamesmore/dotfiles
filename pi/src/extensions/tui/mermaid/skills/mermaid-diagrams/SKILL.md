---
name: mermaid-diagrams
description: 在 Pi 回复中用 Mermaid 画图时，根据要回答的问题选择终端支持的图种，查对应语法和可运行示例，设置标题、说明和主题颜色。
---

# 在 Pi 里画 Mermaid 图

先确定读者要从图里看出什么，再按下表选图。每组参考按图种名称分节；读取所选图种的写法与示例，将其中的参与者、关系或数值换成任务里的内容。回复时，把源码放进顶层的 `mermaid` 围栏，每块一张图，保留图种名称的大小写和 `-beta` 后缀。

## 选图

| 读者要看什么 | 图种与选择依据 |
|---|---|
| [流程与交互]：下一步做什么，谁来做？ | `flowchart` 画步骤和条件分支；`swimlane-beta` 突出跨角色交接；`agentflow-beta` 展示 agent、工具与任务的协作。 |
| 谁先给谁发消息，什么事件改变了状态？ | `sequenceDiagram` 按时间画参与者之间的消息；`zenuml` 用嵌套调用写时序；`stateDiagram-v2` 描述同一对象的状态切换；`eventmodeling` 串起界面、命令、领域事件和读模型。语法见[流程与交互]。 |
| 项目或用户经历了什么？ | `timeline` 列里程碑；`gantt` 画任务的起止时间和依赖；`kanban` 按当前阶段看卡片；`journey` 看用户在各步骤的体验评分；`gitGraph` 看提交、分支和合并。语法见[流程与交互]。 |
| [数据与层次]：比较多少、占比还是流向？ | `xychart-beta` 用柱或线比较数值、趋势；`pie` 看一个整体的构成；`treemap-beta` 看有层级的占比；`sankey-beta` 追踪有数量的流入、分配和汇合。 |
| 比较多个维度、分类或集合？ | `radar-beta` 比较对象在相同指标上的表现；`quadrantChart` 按两个维度定位；`venn-beta` 展示集合交集；`cynefin-beta` 按 Cynefin 决策情境分类。语法见[数据与层次]。 |
| 层级里每条分支代表什么？ | `mindmap` 展开主题；`treeView-beta` 展示目录或父子条目；`ishikawa-beta` 按类别追溯一个结果的成因；`wardley-beta` 标出价值链依赖和组件的演进阶段。语法见[数据与层次]。 |
| [系统与模型]：系统如何组成？ | `architecture-beta` 画服务、分组和连线；`block-beta` 指定块的位置与列布局；`classDiagram` 画类型、成员和继承；`erDiagram` 画实体、字段和关系基数。 |
| 谁使用哪些功能，需求如何验证？ | `usecase-beta` 关联参与者和用例；`requirementDiagram` 关联需求、实现元素和验证方式。语法见[系统与模型]。 |
| 要向谁解释哪一层架构？ | `C4Context` 看系统与用户、外部系统；`C4Container` 看系统内的应用与数据存储；`C4Component` 看一个容器内的组件；`C4Dynamic` 看一次场景里的交互次序；`C4Deployment` 看软件部署在哪些环境和节点。语法见[系统与模型]。 |
| [协议与文法]：字段占几位，输入允许怎么写？ | `packet-beta` 标出位字段；`railroad-beta` 用组合函数构造语法图；已有 ABNF、EBNF 或 PEG 规则时分别用 `railroad-abnf-beta`、`railroad-ebnf-beta`、`railroad-peg-beta`。查看当前渲染器版本用 `info`。 |

## 标题、说明和配色

元数据可选。需要标题或主题时，YAML frontmatter 放在 `mermaid` 围栏**里面**，以两行 `---` 包住，图种声明紧随其后。例如：

```mermaid
---
title: 本月电量去向（兆瓦时）
config:
  themeVariables:
    lineColor: "#70bab6"
    primaryBorderColor: "#87b5e1"
---
sankey-beta
accDescr: 光伏发电共 120 兆瓦时，分别送入电网和储能站。
光伏,电网,90
光伏,储能站,30
```

- `title` 是图上方的标题。图种声明后可用 `accDescr:` 写一行说明，或用 `accDescr { … }` 写多行；Pi 会将说明显示在图下方。
- 要套用主题，在 `config` 下写 `theme: dark`；可选 `base`、`default`、`dark`、`forest`、`neutral`。要沿用终端底色，只设置需要的前景色即可。自定义颜色放在 `config.themeVariables` 下，YAML 中的 `"#rrggbb"` 要带引号。
- 按图种选颜色键。例如饼图用 `pie1`、`pie2` 表示各扇区；XY 图在 `themeVariables` 下嵌套 `xyChart`，其 `plotColorPalette` 是逗号分隔的颜色字符串。全图底色可用 `background` 显式设置。

需要核对一个写法时，把围栏内的内容保存为 `diagram.mmd`，运行 `mermaid-text --width 80 diagram.mmd`。`mermaid-text --example sankey-beta` 能预览该图种的完整内置示例；`mermaid-text --list-types` 列出当前安装版本接受的图种名称和别名。

[流程与交互]: references/workflows.md
[数据与层次]: references/data.md
[系统与模型]: references/systems.md
[协议与文法]: references/formats.md
