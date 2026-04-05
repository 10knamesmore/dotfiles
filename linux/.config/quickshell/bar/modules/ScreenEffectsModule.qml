import "../../theme"
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
    Component.onCompleted: stateReader.running = true
    onClicked: mouse => {
        PanelState.closeAll();
        let pos = root.mapToItem(null, mouse.x, mouse.y);
        PanelState.morphSourceX = pos.x + 2;
        PanelState.morphSourceY = pos.y + 6;
        PanelState.toggleScreenEffects();
    }

    // 定期检查效果状态
    Process {
        id: stateReader

        command: ["cat", Quickshell.env("HOME") + "/.cache/hypr/screen-effects.json"]

        stdout: SplitParser {
            onRead: data => {
                try {
                    let obj = JSON.parse(data);
                    root.effectsActive = (obj.warmth > 0 || obj.grain > 0);
                } catch (e) {}
            }
        }
    }

    Timer {
        interval: 5000
        running: true
        repeat: true
        onTriggered: {
            stateReader.running = false;
            stateReader.running = true;
        }
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
