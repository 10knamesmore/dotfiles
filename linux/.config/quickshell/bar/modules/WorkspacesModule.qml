import "../../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Hyprland._Ipc

// Hyprland 工作区列表，按显示器过滤
Rectangle {
    id: root

    property var barScreen: null

    radius: 16
    color: Colors.base
    implicitHeight: 36
    implicitWidth: row.implicitWidth + 8

    // 柔和阴影
    SoftShadow {
        anchors.fill: parent
        radius: root.radius
    }

    RowLayout {
        id: row

        anchors.centerIn: parent
        spacing: 1

        Repeater {
            // 过滤出属于本显示器的工作区，按 ID 排序
            model: {
                let all = Hyprland.workspaces.values;
                let filtered = all.filter((ws) => {
                    return ws.monitor && root.barScreen && ws.monitor.name === root.barScreen.name;
                });
                filtered.sort((a, b) => {
                    return a.id - b.id;
                });
                return filtered;
            }

            delegate: Rectangle {
                required property var modelData
                property bool isActive: modelData.active || modelData.focused

                width: 35
                height: 28
                radius: 14
                color: isActive ? "transparent" : (wsHover.containsMouse ? Colors.surface1 : "transparent")

                // 激活态渐变 mauve → blue
                Rectangle {
                    anchors.fill: parent
                    radius: parent.radius
                    visible: parent.isActive

                    gradient: Gradient {
                        orientation: Gradient.Horizontal

                        GradientStop {
                            position: 0
                            color: Colors.mauve
                        }

                        GradientStop {
                            position: 1
                            color: Colors.blue
                        }

                    }

                }

                Text {
                    anchors.centerIn: parent
                    z: 1
                    text: modelData.id
                    color: isActive ? Colors.base : (wsHover.containsMouse ? Colors.lavender : Colors.overlay1)
                    font.family: Fonts.family
                    font.pixelSize: Fonts.body
                    font.weight: isActive ? Font.ExtraBold : Font.DemiBold

                    Behavior on color {
                        ColorAnimation {
                            duration: 200
                        }

                    }

                }

                MouseArea {
                    id: wsHover

                    anchors.fill: parent
                    z: 2
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: modelData.activate()
                }

                Behavior on color {
                    ColorAnimation {
                        duration: 200
                    }

                }

            }

        }

    }

}
