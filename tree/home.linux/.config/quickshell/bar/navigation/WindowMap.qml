pragma ComponentBehavior: Bound

import "../../theme"
import "../components"
import QtQuick
import Quickshell
import Quickshell.Hyprland._Ipc

// 窗口块保留空间排列，内部应用图标为 10px（上下堆叠时 8px）。
// 原生窗口对象驱动 delegate，焦点或标题变化不会重建命中区域、打断 hover 动画。
// 布局刷新后，块的位置与高度按 Hyprland windowsMove 弹簧过渡（BarMoveAnimation）。
Item {
    id: root

    required property WindowContext context

    implicitWidth: context.layout.columns > 0 ? context.layout.columns * 27 - 3 : 24
    implicitHeight: 28

    Repeater {
        model: WindowUpdates.toplevels

        delegate: Rectangle {
            id: tile
            required property var modelData
            readonly property var placement: root.context.layout.positions[modelData.address]
            readonly property bool active: modelData === root.context.activeWindow
            readonly property var desktopEntry: DesktopEntries.heuristicLookup(modelData.lastIpcObject.class || "")

            visible: placement !== undefined
            x: placement ? placement.x : 0
            y: placement ? placement.y : 0
            width: 24
            height: placement ? placement.height : 28
            radius: 3
            color: hover.containsMouse ? Colors.withAlpha(Colors.mauve, 0.23)
                : active ? Colors.withAlpha(Colors.mauve, 0.14) : Colors.withAlpha(Colors.surface2, 0.25)
            border.width: 1
            border.color: hover.containsMouse ? Colors.lavender
                : active ? Colors.mauve : Colors.withAlpha(Colors.overlay1, 0.35)

            Image {
                anchors.centerIn: parent
                width: Math.min(tile.placement?.stacked ? 8 : 10, Math.max(0, tile.height - 3))
                height: width
                sourceSize.width: 32
                sourceSize.height: 32
                source: tile.desktopEntry && tile.desktopEntry.icon
                    ? Quickshell.iconPath(tile.desktopEntry.icon, true) : ""
                fillMode: Image.PreserveAspectFit
                smooth: true
            }

            MouseArea {
                id: hover
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    console.info("[navigation] focus window", tile.modelData.address,
                        "on", root.context.barScreen.name);
                    Hyprland.dispatch('hl.dsp.focus({ window = "address:0x' + tile.modelData.address + '" })');
                }
                onWheel: wheel => {
                    const delta = wheel.angleDelta.y;
                    if (delta === 0)
                        return;
                    console.debug("[navigation] resize column", delta > 0 ? "+0.05" : "-0.05");
                    Hyprland.dispatch('hl.dsp.layout("colresize ' + (delta > 0 ? "+" : "-") + '0.05")');
                }
            }

            Behavior on x { BarMoveAnimation {} }
            Behavior on y { BarMoveAnimation {} }
            Behavior on height { BarMoveAnimation {} }
            Behavior on color { BarColorAnimation {} }
            Behavior on border.color { BarColorAnimation {} }
        }
    }

    Text {
        anchors.centerIn: parent
        visible: root.context.layout.columns === 0
        text: "—"
        color: Colors.overlay1
        font.family: Fonts.family
        font.pixelSize: Fonts.small
    }
}
