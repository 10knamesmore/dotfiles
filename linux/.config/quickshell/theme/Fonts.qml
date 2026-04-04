import QtQuick
pragma Singleton

QtObject {
    // 专辑封面占位

    readonly property string family: "Hack Nerd Font"
    // ── 排版尺度 ──
    readonly property int xs: 9
    // 极小标签、状态徽标
    readonly property int caption: 10
    // 时间戳、分区标题、说明文字
    readonly property int small: 11
    // 次要文本、描述
    readonly property int body: 12
    // 正文、表单输入
    readonly property int bodyLarge: 13
    // 强调正文、状态栏模块
    readonly property int icon: 14
    // 行内图标、标准交互元素
    readonly property int title: 15
    // 面板标题
    readonly property int heading: 16
    // 章节标题、搜索栏
    readonly property int iconLarge: 18
    // 大号开关图标
    readonly property int h3: 20
    // 卡片标题
    readonly property int h2: 22
    // 个人资料、媒体控件
    readonly property int h1: 24
    // OSD、天气温度
    readonly property int display3: 32
    // 电源菜单
    readonly property int display2: 36
    // 大号状态图标
    readonly property int display1: 48
    // ── 字重 ──
    readonly property int weightLight: Font.Light
    readonly property int weightNormal: Font.Normal
    readonly property int weightMedium: Font.Medium
    readonly property int weightBold: Font.Bold
    // ── 字间距 ──
    readonly property real trackingNormal: 0
    readonly property real trackingWide: 1.5
    readonly property real trackingXWide: 2.5
}
