import "../theme"
import QtQuick

// 设置面板的分区标题（调节 / 快捷开关 / 主题 / …）——收口 QuickSettings 里重复 6 次的样式。
// 对外只暴露 Text 自带的 text 属性：SectionLabel { text: "调节" }
Text {
    color: Colors.overlay0
    font.family: Fonts.family
    font.pixelSize: Fonts.xs
    font.letterSpacing: 2
    font.weight: Font.Medium
}
