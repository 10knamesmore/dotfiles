import "../../theme"
import QtQuick

// 顶栏各模块共享悬停和状态变色的时长与曲线。
ColorAnimation {
    duration: Tokens.animFast
    easing.type: Easing.BezierSpline
    easing.bezierCurve: Anim.standard
}
