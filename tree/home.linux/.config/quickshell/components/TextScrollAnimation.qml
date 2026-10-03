import QtQuick

// 长文本往返滚动，首尾各停留 1 秒；调用方用 offset 移动文字并决定何时重新开始。
SequentialAnimation {
    id: root

    // 完整文本超出紧凑视口的距离（px）；仍有溢出时更新距离保留进度，回到开头需调用 restart()。
    required property real distance
    readonly property real offset: distance * progress
    property real progress: 0
    // 每段以 45 px/s 计算时长，短距离至少用一秒完成。
    readonly property int travelDuration: Math.max(1000, Math.round(distance / 45 * 1000))

    running: distance > 0
    loops: Animation.Infinite
    onStopped: progress = 0

    PauseAnimation { duration: 1000 }
    NumberAnimation {
        target: root
        property: "progress"
        from: 0
        to: 1
        duration: root.travelDuration
    }
    PauseAnimation { duration: 1000 }
    NumberAnimation {
        target: root
        property: "progress"
        from: 1
        to: 0
        duration: root.travelDuration
    }
}
