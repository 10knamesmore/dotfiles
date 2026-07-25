import "../../theme"
import "../../state"
import "../components"
import QtQuick
import Quickshell
import Quickshell.Io

// 屏幕效果按钮 — 显示当前效果状态，点击切换面板
BarModule {
    id: root

    property bool effectsActive: false

    accentColor: Colors.flamingo
    implicitWidth: label.implicitWidth + 32
    onClicked: mouse => {
        PanelState.closeAll();
        let pos = root.mapToItem(null, mouse.x, mouse.y);
        MorphState.morphSourceX = pos.x + 2;
        MorphState.morphSourceY = pos.y + 6;
        PanelState.toggleScreenEffects();
    }

    // 状态文件仅在 screen_effects.sh / 设置面板写入时才变 —— inotify 监听即可，
    // 取代旧的每 5s fork cat 轮询（per-screen ×2，读同一份全局状态，纯浪费）。
    FileView {
        id: stateFile
        path: Quickshell.env("HOME") + "/.cache/hypr/screen-effects.json"
        watchChanges: true
        printErrors: false
        onLoaded: {
            try {
                let obj = JSON.parse(text());
                root.effectsActive = (obj.warmth > 0 || obj.grain > 0);
            } catch (e) {}
        }
        onFileChanged: reload()
    }

    Text {
        id: label

        anchors.centerIn: parent
        text: root.effectsActive ? "󰌁" : "󰌀"
        color: root.effectsActive ? Colors.flamingo : Colors.overlay1
        font.family: Fonts.family
        font.pixelSize: Fonts.title
        font.weight: Font.DemiBold
    }
}
