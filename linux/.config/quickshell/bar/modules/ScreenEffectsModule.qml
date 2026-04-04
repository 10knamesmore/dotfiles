import QtQuick
import Quickshell
import Quickshell.Io
import "../components"
import "../../theme"

// 屏幕效果按钮 — 显示当前效果状态，点击切换面板
BarModule {
    id: root
    accentColor: Colors.flamingo
    implicitWidth: label.implicitWidth + 32

    property bool effectsActive: false

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

    Component.onCompleted: stateReader.running = true

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
        font.family: "Hack Nerd Font"
        font.pixelSize: 15
        font.weight: Font.DemiBold
    }

    onClicked: {
        PanelState.calendarOpen = false;
        PanelState.mediaOpen = false;
        PanelState.toggleScreenEffects();
    }
}
