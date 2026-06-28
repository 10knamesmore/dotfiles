import "../components"
import "../theme"
import "../state"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

// 显示器管理面板（B 布局：左可拖拽画布 + 右参数控件 + 底部应用/回滚）。
// 状态/IPC 在 MonitorService(常驻) + MonitorState(单例)；本面板只持有编辑态 draft。
PanelOverlay {
    id: root

    // draft：MonitorState.monitors 的可编辑深拷贝；应用前不回写 live 状态
    property var draft: []
    property int selectedIndex: 0
    readonly property var selected: (selectedIndex >= 0 && selectedIndex < draft.length) ? draft[selectedIndex] : null

    function _loadDraft() {
        root.draft = (MonitorState.monitors || []).map(function (m) {
            return {
                "name": m.name, "description": m.description, "enabled": m.enabled,
                "mode": m.mode, "x": m.x, "y": m.y, "scale": m.scale, "transform": m.transform,
                "width": m.width, "height": m.height, "refreshRate": m.refreshRate,
                "availableModes": m.availableModes, "primary": m.primary
            };
        });
        if (root.selectedIndex >= root.draft.length)
            root.selectedIndex = 0;
    }

    function _setField(i, key, val) {
        if (i < 0 || i >= draft.length)
            return;
        draft[i][key] = val;
        root.draft = draft.slice();
    }

    function _setPos(i, x, y) {
        if (i < 0 || i >= draft.length)
            return;
        draft[i].x = x;
        draft[i].y = y;
        root.draft = draft.slice();
    }

    function _setPrimary(i) {
        for (var k = 0; k < draft.length; k++)
            draft[k].primary = (k === i);
        root.draft = draft.slice();
    }

    function _apply() {
        var primary = "";
        var layouts = draft.map(function (d) {
            if (d.primary)
                primary = d.name;
            return { "name": d.name, "enabled": d.enabled, "mode": d.mode, "x": d.x, "y": d.y, "scale": d.scale, "transform": d.transform, "mirror": null };
        });
        MonitorState.requestApply(layouts, primary);
    }

    showing: PanelState.displayOpen
    panelWidth: 720
    panelHeight: root.height * 0.7
    panelTargetX: (root.width - 720) / 2
    panelTargetY: 54
    closedOffsetY: -20
    onCloseRequested: PanelState.displayOpen = false
    onShowingChanged: {
        if (showing) {
            MonitorState.refresh();
            _loadDraft();
        }
    }

    // 面板打开期间，若热插拔/恢复导致 live 状态变化且当前没有未决回滚，则同步 draft
    Connections {
        target: MonitorState
        function onMonitorsChanged() {
            if (root.showing && MonitorState.revertSecs === 0 && !MonitorState.applying)
                root._loadDraft();
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Tokens.spaceL
        spacing: Tokens.spaceM

        // ── 标题 ──
        RowLayout {
            Layout.fillWidth: true
            Text { text: "󰍹"; color: Colors.blue; font.family: Fonts.family; font.pixelSize: Fonts.title }
            Text { text: "显示器"; color: Colors.text; font.family: Fonts.family; font.pixelSize: Fonts.title; font.bold: true }
            Item { Layout.fillWidth: true }
            Rectangle {
                width: 28; height: 28; radius: Tokens.radiusFull
                color: refreshArea.containsMouse ? Colors.surface2 : "transparent"
                Text {
                    anchors.centerIn: parent; text: "󰑓"
                    color: refreshArea.containsMouse ? Colors.blue : Colors.subtext0
                    font.family: Fonts.family; font.pixelSize: Fonts.icon
                }
                MouseArea {
                    id: refreshArea; anchors.fill: parent; hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: { MonitorState.refresh(); root._loadDraft(); }
                }
                Behavior on color { ColorAnimation { duration: Tokens.animFast } }
            }
            Text {
                text: root.draft.length + " 台"
                color: Colors.subtext0; font.family: Fonts.family; font.pixelSize: Fonts.small
            }
        }

        // ── 主体：左画布 + 右控件 ──
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Tokens.spaceM

            MonitorCanvas {
                Layout.fillWidth: true
                Layout.fillHeight: true
                monitors: root.draft
                selectedIndex: root.selectedIndex
                onMonitorSelected: (index) => root.selectedIndex = index
                onMonitorMoved: (index, x, y) => root._setPos(index, x, y)
            }

            Rectangle {
                Layout.preferredWidth: 280
                Layout.fillHeight: true
                radius: Tokens.radiusM
                color: Qt.rgba(Colors.surface0.r, Colors.surface0.g, Colors.surface0.b, Tokens.cardAlpha)
                border.width: 1
                border.color: Qt.rgba(1, 1, 1, 0.06)

                Flickable {
                    anchors.fill: parent
                    anchors.margins: Tokens.spaceM
                    contentHeight: controls.implicitHeight
                    clip: true

                    MonitorControls {
                        id: controls
                        width: parent.width
                        monitor: root.selected
                        onModeEdited: (mode) => root._setField(root.selectedIndex, "mode", mode)
                        onScaleEdited: (scale) => root._setField(root.selectedIndex, "scale", scale)
                        onTransformEdited: (transform) => root._setField(root.selectedIndex, "transform", transform)
                        onPrimaryToggled: root._setPrimary(root.selectedIndex)
                        onEnabledToggled: root._setField(root.selectedIndex, "enabled", !root.selected.enabled)
                    }
                }
            }
        }

        Divider { Layout.fillWidth: true }

        // ── 回滚确认条（应用后倒计时内显示）──
        RevertConfirm {}

        // ── 应用区（无未决回滚时显示）──
        RowLayout {
            Layout.fillWidth: true
            visible: MonitorState.revertSecs === 0
            spacing: Tokens.spaceM

            Text {
                Layout.fillWidth: true
                text: MonitorState.errorMsg !== "" ? ("⚠ " + MonitorState.errorMsg)
                    : (MonitorState.applying ? "正在应用…" : "改动将记入当前显示器组合")
                color: MonitorState.errorMsg !== "" ? Colors.red : Colors.subtext0
                font.family: Fonts.family; font.pixelSize: Fonts.small
                wrapMode: Text.WordWrap
            }

            // 重置
            Rectangle {
                implicitWidth: resetLbl.implicitWidth + Tokens.spaceL; implicitHeight: 30
                radius: Tokens.radiusS
                color: resetArea.containsMouse ? Colors.surface2 : Colors.surface1
                Text { id: resetLbl; anchors.centerIn: parent; text: "重置"; color: Colors.text; font.family: Fonts.family; font.pixelSize: Fonts.small }
                MouseArea {
                    id: resetArea; anchors.fill: parent; hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root._loadDraft()
                }
                Behavior on color { ColorAnimation { duration: Tokens.animFast } }
            }

            // 应用
            Rectangle {
                implicitWidth: applyLbl.implicitWidth + Tokens.spaceL; implicitHeight: 30
                radius: Tokens.radiusS
                color: applyArea.containsMouse ? Qt.lighter(Colors.blue, 1.1) : Colors.blue
                Text { id: applyLbl; anchors.centerIn: parent; text: "应用"; color: Colors.base; font.family: Fonts.family; font.pixelSize: Fonts.small; font.bold: true }
                MouseArea {
                    id: applyArea; anchors.fill: parent; hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root._apply()
                }
                Behavior on color { ColorAnimation { duration: Tokens.animFast } }
            }
        }
    }
}
