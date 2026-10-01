import "../../theme"
import QtQuick

// 顶栏内容宽度变化统一沿用胶囊的减速过渡。
NumberAnimation {
    duration: Tokens.animSlow
    easing.type: Easing.OutCubic
}
