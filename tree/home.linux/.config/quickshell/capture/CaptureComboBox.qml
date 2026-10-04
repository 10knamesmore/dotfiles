import "../theme"
import QtQuick.Controls

// 来源、颜色格式与笔宽共用浮层调色板，保留原生 ComboBox 的选择语义。
ComboBox {
    palette.button: Colors.surface0
    palette.buttonText: Colors.text
    palette.base: Colors.base
    palette.text: Colors.text
    palette.window: Colors.base
    palette.windowText: Colors.text
    palette.highlight: Colors.blue
    palette.highlightedText: Colors.crust
    font.pixelSize: 13
}
