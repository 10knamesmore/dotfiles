import "../theme"
import QtQuick

// 控制中心的分区标题。
// 对外只暴露 Text 自带的 text 属性：SectionLabel { text: "调节" }
Text {
    color: Colors.overlay0
    font.family: Fonts.family
    font.pixelSize: Fonts.xs
    font.letterSpacing: 2
    font.weight: Font.Medium
}
