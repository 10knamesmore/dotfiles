import "../../theme"
import "../../state"
import "../components"
import QtQuick
import Quickshell

// 屏幕效果按钮 — 显示当前效果状态，点击切换面板
BarModule {
    id: root

    // 直接绑 State 单例。过去是 inotify 监听 ~/.cache/hypr/screen-effects.json，
    // 但写入方（ScreenEffectsService）和本模块在同一个 quickshell 进程里，
    // 等于「自己写文件 → 内核通知自己 → 自己读回来」，纯绕路。
    readonly property bool effectsActive: ScreenEffectsState.effectsActive

    accentColor: Colors.flamingo
    implicitWidth: label.implicitWidth + 32
    onClicked: mouse => {
        PanelState.closeAll();
        MorphState.openFrom(root, () => PanelState.toggleScreenEffects());
    }

    Row {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        spacing: 8

        Text {
            id: label
            text: root.effectsActive ? "󰌁" : "󰌀"
            color: root.effectsActive ? Colors.flamingo : Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.title
            font.weight: Font.DemiBold
        }

        Text {
            text: "屏幕效果"
            opacity: root.expansion
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}
