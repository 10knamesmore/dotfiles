## pie

用 `pie` 开始，后续每行以 `"类别": 数值` 声明扇区；`showData` 会在图例中显示数值。

```mermaid
pie showData
    "步行" : 45
    "公交" : 35
    "骑行" : 20
```

## xychart-beta

`x-axis` 列出离散类别，`y-axis` 可用 `最小值 --> 最大值` 指定数值范围；`bar` 和 `line` 的数据项数量应与横轴类别对应。

```mermaid
xychart-beta
    x-axis ["一月", "二月", "三月"]
    y-axis "降雨量（毫米）" 0 --> 100
    bar "降雨量" [35, 62, 48]
    line "目标值" [50, 50, 50]
```

## quadrantChart

用 `x-axis`、`y-axis` 设置轴端标签，`quadrant-1` 到 `quadrant-4` 设置象限名称；数据点坐标是 0 到 1 之间的 `[x, y]`。

```mermaid
quadrantChart
    x-axis 成本低 --> 成本高
    y-axis 收益低 --> 收益高
    quadrant-1 逐步推进
    quadrant-2 优先投入
    quadrant-3 暂缓
    quadrant-4 重新评估
    屋顶光伏: [0.35, 0.82]
    储能扩容: [0.78, 0.74]
    灯具改造: [0.24, 0.38]
```

## radar-beta

`axis` 用逗号分隔维度，`max`、`min` 设置数值范围，`ticks` 设置刻度数；每条 `curve` 的花括号数值按轴顺序填写。

```mermaid
radar-beta
    axis 成本,效率,可靠性,易用性
    max 10
    min 0
    ticks 5
    curve 方案甲 {7,8,9,6}
    curve 方案乙 {9,6,7,8}
```

## sankey-beta

每行按「来源, 去向, 数值」声明一条流，数值表示流量，单位由图表数据自行约定；同名来源或去向会汇入同一节点。

```mermaid
sankey-beta
太阳能,电网,70
太阳能,储能,30
风能,电网,60
储能,电网,20
电网,住户,120
电网,工厂,30
```

## treemap-beta

每个顶层名称声明一个分组，缩进的子项以 `名称: 数值` 给出面积权重；数值可用下划线分隔千位。

```mermaid
treemap-beta
"运营"
  "人员": 4_800
  "设施": 1_200
"项目"
  "培训": 900
  "设备": 600
```

## venn-beta

用 `set 名称 [标签]: 数值` 声明集合，`union 集合,集合 [标签]: 数值` 标注重叠区域；`text 集合,集合 文字 [标签]` 可在交集处添加说明。

```mermaid
venn-beta
set 公交 [公交乘客]: 40
set 地铁 [地铁乘客]: 50
union 公交,地铁 [两者皆用]: 18
text 公交,地铁 换乘通勤者 [常换乘]
```

## mindmap

根节点可写成 `标识((文字))`，后续节点通过逐级缩进表示父子层级；节点内容可直接使用中文。

```mermaid
mindmap
    root((社区菜园))
        种植
            选种
            轮作
        维护
            浇水
            堆肥
```

## treeView-beta

用缩进声明树形层级；节点文字可用双引号包裹，并以 `##` 添加描述。

```mermaid
treeView-beta
"项目" ## 工作区
  "应用" ## 可部署服务
    "前端" ## 用户界面
    "后端" ## API 服务
  "文档" ## 使用与设计说明
```

## ishikawa-beta

首行写待分析的问题，后续缩进列出原因类别及其子因；继续增加缩进即可细分原因。

```mermaid
ishikawa-beta
交付延迟增加
  人员
    排班不足
    新人培训不完整
  流程
    审批等待过长
    需求频繁变更
  设备
    分拣机故障
```

## wardley-beta

以 `anchor` 和 `component` 按 `[可见性, 演进度]` 放置节点，坐标范围为 0 到 1；`evolution` 定义演进阶段，箭头表示依赖关系。

```mermaid
wardley-beta
evolution 探索 -> 定制 -> 产品 -> 通用
anchor 用户需求 [0.95, 0.45]
component 推荐服务 [0.68, 0.35]
component 商品目录 [0.42, 0.72]
用户需求 -> 推荐服务
推荐服务 -> 商品目录
```

## cynefin-beta

用 `clear`、`complicated`、`complex`、`chaotic`、`confusion` 声明领域及其中事项；`-->` 可描述事项在领域间的转移，并在冒号后附加说明。

```mermaid
cynefin-beta
  clear
    "按计划补货"
  complicated
    "评估新仓库"
  complex
    "试行社区配送"
  chaotic
    "先处理断电"
  confusion
    "缺货原因待查"
  confusion --> complicated : "核对库存记录"
  chaotic --> complex : "恢复供电"
```
