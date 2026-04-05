import "../components"
import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

// 剪贴板历史面板 — 右上角弹出
PanelOverlay {
    id: root

    property string searchQuery: ""

    function loadClipboard() {
        clipModel.clear();
        filteredModel.clear();
        clipListProc.running = true;
    }

    function applyFilter() {
        filteredModel.clear();
        let q = searchQuery.toLowerCase();
        for (let i = 0; i < clipModel.count; i++) {
            let item = clipModel.get(i);
            if (q.length === 0 || item.preview.toLowerCase().includes(q))
                filteredModel.append(item);

        }
    }

    function selectItem(clipId) {
        copyProc.command = ["sh", "-c", "cliphist decode " + clipId + " | wl-copy"];
        copyProc.running = true;
        PanelState.clipboardOpen = false;
    }

    function deleteItem(clipId) {
        deleteProc.command = ["cliphist", "delete-query", clipId];
        deleteProc.running = true;
        // 从两个 model 中移除
        for (let i = filteredModel.count - 1; i >= 0; i--) {
            if (filteredModel.get(i).clipId === clipId) {
                filteredModel.remove(i);
                break;
            }
        }
        for (let i = clipModel.count - 1; i >= 0; i--) {
            if (clipModel.get(i).clipId === clipId) {
                clipModel.remove(i);
                break;
            }
        }
    }

    showing: PanelState.clipboardOpen
    panelWidth: 400
    panelHeight: root.height * 0.6
    panelTargetX: root.width - 410
    panelTargetY: 54
    closedOffsetY: -20
    onCloseRequested: PanelState.clipboardOpen = false
    onShowingChanged: {
        if (showing) {
            searchQuery = "";
            loadClipboard();
            focusTimer.start();
        }
    }

    ListModel {
        id: clipModel
    }

    ListModel {
        id: filteredModel
    }

    Timer {
        id: focusTimer

        interval: 50
        onTriggered: searchInput.forceActiveFocus()
    }

    // ── 进程 ──
    Process {
        id: clipListProc

        command: ["cliphist", "list"]
        onExited: root.applyFilter()

        stdout: SplitParser {
            onRead: (data) => {
                // 格式: "id\tpreview text"
                let tab = data.indexOf("\t");
                if (tab < 0)
                    return ;

                let id = data.substring(0, tab).trim();
                let preview = data.substring(tab + 1).trim();
                if (preview.length > 0)
                    clipModel.append({
                    "clipId": id,
                    "preview": preview
                });

            }
        }

    }

    Process {
        id: copyProc
    }

    Process {
        id: deleteProc
    }

    Process {
        id: wipeProc

        command: ["cliphist", "wipe"]
        onExited: {
            clipModel.clear();
            filteredModel.clear();
        }
    }

    // ── UI ──
    ColumnLayout {
        id: col

        anchors.fill: parent
        anchors.margins: Tokens.spaceL
        spacing: Tokens.spaceS

        // 标题栏
        RowLayout {
            Layout.fillWidth: true

            Text {
                text: "󰅍 剪贴板"
                font.family: Fonts.family
                font.pixelSize: Fonts.title
                font.bold: true
                color: Colors.text
            }

            Item {
                Layout.fillWidth: true
            }

            Text {
                visible: filteredModel.count > 0
                text: filteredModel.count + " 条"
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }

            Rectangle {
                visible: clipModel.count > 0
                width: clearText.implicitWidth + 16
                height: 26
                radius: Tokens.radiusFull
                color: clearArea.containsMouse ? Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.15) : "transparent"

                Text {
                    id: clearText

                    anchors.centerIn: parent
                    text: "清空"
                    color: clearArea.containsMouse ? Colors.red : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small

                    Behavior on color {
                        ColorAnimation {
                            duration: Tokens.animFast
                            easing.type: Easing.BezierSpline
                            easing.bezierCurve: Anim.standard
                        }

                    }

                }

                MouseArea {
                    id: clearArea

                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: wipeProc.running = true
                }

                Behavior on color {
                    ColorAnimation {
                        duration: Tokens.animFast
                        easing.type: Easing.BezierSpline
                        easing.bezierCurve: Anim.standard
                    }

                }

            }

        }

        // 搜索框
        Rectangle {
            Layout.fillWidth: true
            height: 36
            radius: Tokens.radiusMS
            color: Colors.surface1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 10
                anchors.rightMargin: 10
                spacing: Tokens.spaceS

                Text {
                    text: ""
                    color: Colors.overlay1
                    font.family: Fonts.family
                    font.pixelSize: Fonts.icon
                }

                TextInput {
                    id: searchInput

                    Layout.fillWidth: true
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.bodyLarge
                    clip: true
                    selectByMouse: true
                    onTextChanged: {
                        root.searchQuery = text;
                        root.applyFilter();
                    }
                    Keys.onEscapePressed: PanelState.clipboardOpen = false

                    Text {
                        anchors.fill: parent
                        text: "搜索..."
                        color: Colors.overlay0
                        font: parent.font
                        visible: !parent.text && !parent.activeFocus
                    }

                }

            }

        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: Colors.surface1
        }

        // 空状态
        Text {
            visible: filteredModel.count === 0
            text: clipModel.count === 0 ? "剪贴板为空" : "未找到匹配项"
            color: Colors.overlay0
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: 20
            Layout.bottomMargin: 20
        }

        // 列表
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: filteredModel
            spacing: Tokens.spaceXS
            clip: true

            delegate: Rectangle {
                required property int index
                required property string clipId
                required property string preview

                width: ListView.view.width
                height: 40
                radius: Tokens.radiusS
                color: itemHover.containsMouse ? Colors.surface1 : "transparent"

                MouseArea {
                    id: itemHover

                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    acceptedButtons: Qt.LeftButton
                    onClicked: root.selectItem(clipId)
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 10
                    anchors.rightMargin: 6
                    spacing: Tokens.spaceS

                    Text {
                        text: preview
                        color: Colors.text
                        font.family: Fonts.family
                        font.pixelSize: Fonts.body
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                        maximumLineCount: 1
                    }

                    // 删除按钮
                    Rectangle {
                        Layout.preferredWidth: 28
                        Layout.preferredHeight: 28
                        radius: Tokens.radiusFull
                        color: delArea.containsMouse ? Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.15) : "transparent"

                        Text {
                            anchors.centerIn: parent
                            text: "󰅖"
                            color: delArea.containsMouse ? Colors.red : Colors.overlay0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.icon

                            Behavior on color {
                                ColorAnimation {
                                    duration: Tokens.animFast
                                    easing.type: Easing.BezierSpline
                                    easing.bezierCurve: Anim.standard
                                }

                            }

                        }

                        MouseArea {
                            id: delArea

                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.deleteItem(clipId)
                        }

                        Behavior on color {
                            ColorAnimation {
                                duration: Tokens.animFast
                                easing.type: Easing.BezierSpline
                                easing.bezierCurve: Anim.standard
                            }

                        }

                    }

                }

                Behavior on color {
                    ColorAnimation {
                        duration: Tokens.animFast
                    }

                }

            }

        }

    }

}
