import "../components"
import "../theme"
import "../state"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

// Quick Settings — 左侧滑出面板
PanelOverlay {
    id: root

    // ── 系统状态 ──
    // 电源档位（power-profiles-daemon 的 ActiveProfile），由面板开关切换，daemon 自己持久化
    property string powerProfile: "balanced"

    function refreshStatus() {
        powerProfileProc.running = true;
    }

    // 均衡 ↔ 性能：只动 PPD 的 ActiveProfile，它会把 platform_profile 和 EPP 一起切
    function togglePerformanceMode() {
        let target = root.powerProfile === "performance" ? "balanced" : "performance";
        powerProfileSetProc.command = ["busctl", "--system", "set-property",
            "net.hadess.PowerProfiles", "/net/hadess/PowerProfiles",
            "net.hadess.PowerProfiles", "ActiveProfile", "s", target];
        powerProfileSetProc.running = true;
    }

    showing: PanelState.settingsOpen
    entrance: PanelOverlay.Slide
    panelWidth: 340
    panelHeight: root.height - 64
    panelTargetX: 10
    panelTargetY: 54
    closedOffsetX: -360
    closedOffsetY: 0
    onCloseRequested: PanelState.settingsOpen = false
    onShowingChanged: {
        if (showing)
            refreshStatus();

    }

    // ── 进程 ──
    Process {
        id: actionProc
    }

    // 电源档位：power-profiles-daemon，D-Bus property 读写等价 powerprofilesctl get/set
    //（用 busctl 免掉 python-gobject 可选依赖，busctl 是 systemd 自带）
    Process {
        id: powerProfileProc

        command: ["busctl", "--system", "get-property",
            "net.hadess.PowerProfiles", "/net/hadess/PowerProfiles",
            "net.hadess.PowerProfiles", "ActiveProfile"]

        stdout: SplitParser {
            onRead: (data) => {
                let m = data.match(/"([^"]+)"/);
                if (m)
                    root.powerProfile = m[1];
            }
        }

    }

    Process {
        id: powerProfileSetProc

        // 无论成败都回读，以 daemon 的实际状态为准
        onExited: {
            powerProfileProc.running = false;
            powerProfileProc.running = true;
        }

    }

    // ── UI ──
    Flickable {
        anchors.fill: parent
        anchors.margins: Tokens.spaceL
        contentHeight: mainCol.implicitHeight
        clip: true

        ColumnLayout {
            id: mainCol

            // ── 系统信息（可点击展开）──
            property bool infoExpanded: false

            width: parent.width
            spacing: Tokens.spaceM

            // ── 用户头像 ──
            ProfileHeader {
                Layout.fillWidth: true
            }

            Divider {
                Layout.fillWidth: true
            }

            // ── 开关区 ──
            SectionLabel {
                text: "快捷开关"
            }

            GridLayout {
                Layout.fillWidth: true
                columns: 2
                rowSpacing: 8
                columnSpacing: 8

                QuickToggle {
                    icon: "󰓅"
                    label: "性能"
                    status: root.powerProfile === "performance" ? "性能模式" : "均衡模式"
                    toggled: root.powerProfile === "performance"
                    onClicked: root.togglePerformanceMode()
                }

                QuickToggle {
                    icon: "󰈋"
                    label: "取色器"
                    status: "hyprpicker"
                    toggled: false
                    onClicked: {
                        PanelState.settingsOpen = false;
                        actionProc.command = ["hyprpicker", "-a"];
                        actionProc.running = true;
                    }
                }

            }

            Divider {
                Layout.fillWidth: true
            }

            // ── 截图 ──
            SectionLabel {
                text: "工具"
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Tokens.spaceS

                ToolButton {
                    icon: "󰹑"
                    label: "区域截图"
                    command: "hyprshot -m region"
                }

                ToolButton {
                    icon: "󰖯"
                    label: "窗口截图"
                    command: "hyprshot -m window"
                }

            }

            // ── 显示器设置 ──（打开可视化显示器管理面板）
            RowLayout {
                Layout.fillWidth: true
                spacing: Tokens.spaceS

                ToolButton {
                    icon: "󰍹"
                    label: "显示器设置"
                    onClicked: PanelState.toggleDisplay()
                }
            }

            Divider {
                Layout.fillWidth: true
            }

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: sysInfoCol.implicitHeight + 20
                radius: Tokens.radiusM
                color: sysHover.containsMouse ? Colors.withAlpha(Colors.surface1, Tokens.cardAlpha) : Colors.withAlpha(Colors.surface0, Tokens.cardAlpha)
                border.color: sysHover.containsMouse ? Colors.withAlpha(Colors.blue, Tokens.borderHoverAlpha) : Colors.overlay(0.06)
                border.width: 1
                clip: true

                MouseArea {
                    id: sysHover

                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: mainCol.infoExpanded = !mainCol.infoExpanded
                }

                ColumnLayout {
                    id: sysInfoCol

                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: 10
                    spacing: Tokens.spaceS

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 10

                        Text {
                            text: "󰍹"
                            color: Colors.overlay1
                            font.family: Fonts.family
                            font.pixelSize: Fonts.icon
                        }

                        Text {
                            id: uptimeText

                            color: Colors.subtext0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.small
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
                                    onRead: (data) => {
                                        return uptimeText.text = data;
                                    }
                                }

                            }

                        }

                        Item {
                            Layout.fillWidth: true
                        }

                        Text {
                            text: mainCol.infoExpanded ? "󰅃" : "󰅀"
                            color: Colors.overlay1
                            font.family: Fonts.family
                            font.pixelSize: Fonts.icon
                        }

                        // 重载按钮（阻止点击穿透到卡片）
                        Rectangle {
                            width: 28
                            height: 28
                            radius: Tokens.radiusFull
                            color: reloadHover.containsMouse ? Colors.surface2 : "transparent"

                            Text {
                                anchors.centerIn: parent
                                text: "󰑓"
                                color: Colors.subtext0
                                font.family: Fonts.family
                                font.pixelSize: Fonts.icon
                            }

                            MouseArea {
                                id: reloadHover

                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: (mouse) => {
                                    mouse.accepted = true;
                                    Quickshell.reload(true);
                                }
                            }

                            Behavior on color {
                                ColorAnimation {
                                    duration: 150
                                }

                            }

                        }

                    }

                    // 折叠的系统信息
                    SystemInfo {
                        Layout.fillWidth: true
                        expanded: mainCol.infoExpanded
                    }

                }

                Behavior on color {
                    ColorAnimation {
                        duration: 200
                        easing.type: Easing.OutCubic
                    }

                }

                Behavior on border.color {
                    ColorAnimation {
                        duration: 200
                        easing.type: Easing.OutCubic
                    }

                }

                Behavior on implicitHeight {
                    NumberAnimation {
                        duration: 200
                        easing.type: Easing.OutCubic
                    }

                }

            }

            Divider {
                Layout.fillWidth: true
            }

            // ── 电源操作 ──
            SectionLabel {
                text: "电源"
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Tokens.spaceS

                PowerButton {
                    icon: "󰌾"
                    label: "锁屏"
                    command: "hyprlock"
                }

                PowerButton {
                    icon: "󰍃"
                    label: "注销"
                    command: "hyprctl dispatch 'hl.dsp.exit()'"
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
