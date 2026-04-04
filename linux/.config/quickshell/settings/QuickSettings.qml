import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "../theme"

// Quick Settings — 左侧滑出面板
PanelWindow {
    id: root

    anchors.top: true
    anchors.bottom: true
    anchors.left: true
    anchors.right: true

    property bool showing: PanelState.settingsOpen
    property bool animating: _opacityAnim.running || _slideAnim.running
    visible: showing || animating

    focusable: root.showing
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"

    // ── 系统状态 ──
    property bool wifiEnabled: true
    property bool btEnabled: true
    property string wifiName: ""
    property string btDevice: ""
    property int brightnessValue: 100
    property int volumePct: 0
    property bool volumeMuted: false

    onShowingChanged: {
        if (showing)
            refreshStatus();
    }

    function refreshStatus() {
        wifiProc.running = true;
        btPowerProc.running = true;
        btDeviceProc.running = true;
        brightnessProc.running = true;
    }

    // ── 进程 ──
    Process {
        id: wifiProc
        command: ["nmcli", "radio", "wifi"]
        stdout: SplitParser {
            onRead: data => root.wifiEnabled = data.trim() === "enabled"
        }
    }
    Process {
        id: wifiNameProc
        command: ["nmcli", "-t", "-f", "active,ssid", "dev", "wifi"]
        stdout: SplitParser {
            onRead: data => {
                if (data.startsWith("yes:"))
                    root.wifiName = data.substring(4);
            }
        }
    }
    Process {
        id: btPowerProc
        command: ["bluetoothctl", "show"]
        stdout: SplitParser {
            onRead: data => {
                if (data.includes("Powered:"))
                    root.btEnabled = data.includes("yes");
            }
        }
    }
    Process {
        id: btDeviceProc
        command: ["bluetoothctl", "devices", "Connected"]
        stdout: SplitParser {
            onRead: data => {
                let parts = data.split(" ");
                if (parts.length >= 3)
                    root.btDevice = parts.slice(2).join(" ");
            }
        }
    }
    Process {
        id: brightnessProc
        command: ["brightnessctl", "-m"]
        stdout: SplitParser {
            onRead: data => {
                let parts = data.split(",");
                if (parts.length >= 4)
                    root.brightnessValue = parseInt(parts[3]) || 0;
            }
        }
    }
    Process {
        id: volProc
        command: ["wpctl", "get-volume", "@DEFAULT_AUDIO_SINK@"]
        stdout: SplitParser {
            onRead: data => {
                let m = data.match(/Volume:\s+([\d.]+)(\s+\[MUTED\])?/);
                if (m) {
                    root.volumePct = Math.round(parseFloat(m[1]) * 100);
                    root.volumeMuted = m[2] !== undefined;
                }
            }
        }
    }
    Process {
        id: actionProc
    }

    Timer {
        running: root.showing
        interval: 40
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            volProc.running = true;
        }
    }

    // ── UI ──
    Rectangle {
        anchors.fill: parent
        color: "#000000"
        opacity: root.showing ? 0.15 : 0.0
        Behavior on opacity {
            NumberAnimation {
                duration: 250
                easing.type: Easing.OutCubic
            }
        }
    }

    Item {
        focus: root.showing
        Keys.onEscapePressed: PanelState.settingsOpen = false
    }

    MouseArea {
        anchors.fill: parent
        onClicked: PanelState.settingsOpen = false
    }

    Rectangle {
        id: panel
        width: 340
        anchors {
            top: parent.top
            bottom: parent.bottom
            left: parent.left
            topMargin: 54
            bottomMargin: 10
        }
        anchors.leftMargin: root.showing ? 10 : -360
        radius: 16
        color: Qt.rgba(Colors.surface0.r, Colors.surface0.g, Colors.surface0.b, 0.85)
        border.color: Qt.rgba(1, 1, 1, 0.08)
        border.width: 1
        clip: true

        Behavior on anchors.leftMargin {
            NumberAnimation {
                id: _slideAnim
                duration: 250
                easing.type: Easing.OutCubic
            }
        }
        opacity: root.showing ? 1.0 : 0.0
        Behavior on opacity {
            NumberAnimation {
                id: _opacityAnim
                duration: 250
                easing.type: Easing.OutCubic
            }
        }

        MouseArea {
            anchors.fill: parent
            onClicked: mouse => mouse.accepted = true
        }

        Flickable {
            anchors.fill: parent
            anchors.margins: 16
            contentHeight: mainCol.implicitHeight
            clip: true

            ColumnLayout {
                id: mainCol
                width: parent.width
                spacing: 12

                // ── 标题 ──
                Text {
                    text: "快捷设置"
                    color: Colors.text
                    font.family: "Hack Nerd Font"
                    font.pixelSize: 16
                    font.weight: Font.Bold
                }

                Rectangle {
                    Layout.fillWidth: true
                    height: 1
                    color: Colors.surface1
                    opacity: 0.5
                }

                // ── 滑块区 ──
                SettingsSlider {
                    Layout.fillWidth: true
                    icon: root.brightnessValue > 50 ? "󰃠" : "󰃞"
                    value: root.brightnessValue / 100.0
                    label: "亮度 " + root.brightnessValue + "%"
                    accentColor: Colors.yellow
                    onMoved: val => {
                        root.brightnessValue = Math.round(val * 100);
                        actionProc.command = [Quickshell.env("HOME") + "/dotfiles/generated/scripts/hypr/screen_effects.sh", "brightness", String(root.brightnessValue)];
                        actionProc.running = true;
                    }
                }
                SettingsSlider {
                    Layout.fillWidth: true
                    icon: root.volumeMuted ? "" : (root.volumePct < 50 ? "" : "")
                    value: root.volumePct / 100.0
                    label: "音量 " + root.volumePct + "%"
                    accentColor: Colors.blue
                    onMoved: val => {
                        actionProc.command = ["wpctl", "set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", Math.round(val * 100) + "%"];
                        actionProc.running = true;
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    height: 1
                    color: Colors.surface1
                    opacity: 0.5
                }

                // ── 开关区 ──
                GridLayout {
                    Layout.fillWidth: true
                    columns: 3
                    rowSpacing: 8
                    columnSpacing: 8

                    QuickToggle {
                        icon: root.wifiEnabled ? "󰤨" : "󰤭"
                        label: "WiFi"
                        status: root.wifiName || (root.wifiEnabled ? "已开启" : "已关闭")
                        toggled: root.wifiEnabled
                        onClicked: {
                            actionProc.command = ["nmcli", "radio", "wifi", root.wifiEnabled ? "off" : "on"];
                            actionProc.running = true;
                            root.wifiEnabled = !root.wifiEnabled;
                        }
                    }
                    QuickToggle {
                        icon: root.btEnabled ? "󰂯" : "󰂲"
                        label: "蓝牙"
                        status: root.btDevice || (root.btEnabled ? "已开启" : "已关闭")
                        toggled: root.btEnabled
                        onClicked: {
                            actionProc.command = ["bluetoothctl", "power", root.btEnabled ? "off" : "on"];
                            actionProc.running = true;
                            root.btEnabled = !root.btEnabled;
                        }
                    }
                    QuickToggle {
                        icon: PanelState.dndEnabled ? "󰂛" : "󰂚"
                        label: "勿扰"
                        status: PanelState.dndEnabled ? "已开启" : "已关闭"
                        toggled: PanelState.dndEnabled
                        onClicked: PanelState.dndEnabled = !PanelState.dndEnabled
                    }
                    QuickToggle {
                        icon: root.volumeMuted ? "" : "󰕾"
                        label: "静音"
                        status: root.volumeMuted ? "已静音" : "未静音"
                        toggled: root.volumeMuted
                        onClicked: {
                            actionProc.command = ["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"];
                            actionProc.running = true;
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    height: 1
                    color: Colors.surface1
                    opacity: 0.5
                }

                // ── 系统信息 ──
                property bool infoExpanded: false

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10

                    Text {
                        text: "󰍹"
                        color: Colors.overlay1
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 14
                    }
                    Text {
                        id: uptimeText
                        color: Colors.subtext0
                        font.family: "Hack Nerd Font"
                        font.pixelSize: 11
                        text: "uptime..."
                        Timer {
                            running: root.showing
                            interval: 60000
                            repeat: true
                            triggeredOnStart: true
                            onTriggered: uptimeProc.running = true
                        }
                        Process {
                            id: uptimeProc
                            command: ["sh", "-c", "uptime -p | sed 's/up //'"]
                            stdout: SplitParser {
                                onRead: data => uptimeText.text = data
                            }
                        }
                    }
                    Item {
                        Layout.fillWidth: true
                    }

                    // 展开/折叠按钮
                    Rectangle {
                        width: 40
                        height: 40
                        radius: 20
                        color: expandHover.containsMouse ? Colors.surface1 : "transparent"
                        Behavior on color {
                            ColorAnimation {
                                duration: 150
                            }
                        }
                        Text {
                            anchors.centerIn: parent
                            text: mainCol.infoExpanded ? "󰅃" : "󰅀"
                            color: Colors.subtext0
                            font.family: "Hack Nerd Font"
                            font.pixelSize: 18
                        }
                        MouseArea {
                            id: expandHover
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: mainCol.infoExpanded = !mainCol.infoExpanded
                        }
                    }

                    // 重载按钮
                    Rectangle {
                        width: 40
                        height: 40
                        radius: 20
                        color: reloadHover.containsMouse ? Colors.surface1 : "transparent"
                        Behavior on color {
                            ColorAnimation {
                                duration: 150
                            }
                        }
                        Text {
                            anchors.centerIn: parent
                            text: "󰑓"
                            color: Colors.subtext0
                            font.family: "Hack Nerd Font"
                            font.pixelSize: 18
                        }
                        MouseArea {
                            id: reloadHover
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: Quickshell.reload(true)
                        }
                    }
                }

                // 折叠的系统信息
                SystemInfo {
                    Layout.fillWidth: true
                    expanded: mainCol.infoExpanded
                }

                Rectangle {
                    Layout.fillWidth: true
                    height: 1
                    color: Colors.surface1
                    opacity: 0.5
                }

                // ── 电源操作 ──
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    PowerButton {
                        icon: "󰌾"
                        label: "锁屏"
                        command: "hyprlock"
                    }
                    PowerButton {
                        icon: "󰍃"
                        label: "注销"
                        command: "hyprctl dispatch exit"
                    }
                    PowerButton {
                        icon: "󰤄"
                        label: "挂起"
                        command: "systemctl suspend"
                    }
                    PowerButton {
                        icon: "󰜉"
                        label: "重启"
                        command: "systemctl reboot"
                    }
                    PowerButton {
                        icon: "󰐥"
                        label: "关机"
                        command: "systemctl poweroff"
                    }
                }
            }
        }
    }
}
