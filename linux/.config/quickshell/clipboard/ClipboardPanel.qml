import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "../theme"

// 剪贴板历史面板 — 右上角弹出
PanelWindow {
    id: root

    anchors.top: true
    anchors.bottom: true
    anchors.left: true
    anchors.right: true

    property bool showing: PanelState.clipboardOpen
    property bool animating: _opacityAnim.running || _slideAnim.running
    visible: showing || animating

    focusable: root.showing
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"

    property string searchQuery: ""

    ListModel { id: clipModel }
    ListModel { id: filteredModel }

    onShowingChanged: {
        if (showing) {
            searchQuery = ""
            loadClipboard()
            focusTimer.start()
        }
    }

    Timer {
        id: focusTimer
        interval: 50
        onTriggered: searchInput.forceActiveFocus()
    }

    function loadClipboard() {
        clipModel.clear()
        filteredModel.clear()
        clipListProc.running = true
    }

    function applyFilter() {
        filteredModel.clear()
        let q = searchQuery.toLowerCase()
        for (let i = 0; i < clipModel.count; i++) {
            let item = clipModel.get(i)
            if (q.length === 0 || item.preview.toLowerCase().includes(q)) {
                filteredModel.append(item)
            }
        }
    }

    function selectItem(clipId) {
        copyProc.command = ["sh", "-c", "cliphist decode " + clipId + " | wl-copy"]
        copyProc.running = true
        PanelState.clipboardOpen = false
    }

    function deleteItem(clipId) {
        deleteProc.command = ["cliphist", "delete-query", clipId]
        deleteProc.running = true
        // 从两个 model 中移除
        for (let i = filteredModel.count - 1; i >= 0; i--) {
            if (filteredModel.get(i).clipId === clipId) { filteredModel.remove(i); break }
        }
        for (let i = clipModel.count - 1; i >= 0; i--) {
            if (clipModel.get(i).clipId === clipId) { clipModel.remove(i); break }
        }
    }

    // ── 进程 ──
    Process {
        id: clipListProc
        command: ["cliphist", "list"]
        stdout: SplitParser {
            onRead: data => {
                // 格式: "id\tpreview text"
                let tab = data.indexOf("\t")
                if (tab < 0) return
                let id = data.substring(0, tab).trim()
                let preview = data.substring(tab + 1).trim()
                if (preview.length > 0) {
                    clipModel.append({ clipId: id, preview: preview })
                }
            }
        }
        onExited: root.applyFilter()
    }
    Process { id: copyProc }
    Process { id: deleteProc }
    Process {
        id: wipeProc
        command: ["cliphist", "wipe"]
        onExited: { clipModel.clear(); filteredModel.clear() }
    }

    // ── UI ──
    Rectangle {
        anchors.fill: parent; color: "#000000"
        opacity: root.showing ? 0.15 : 0.0
        Behavior on opacity { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }
    }

    Item { focus: root.showing; Keys.onEscapePressed: PanelState.clipboardOpen = false }
    MouseArea { anchors.fill: parent; onClicked: PanelState.clipboardOpen = false }

    Rectangle {
        id: panel
        width: 400
        height: root.height * 0.6
        radius: 16
        color: Qt.rgba(Colors.surface0.r, Colors.surface0.g, Colors.surface0.b, 0.85)
        border.color: Qt.rgba(1, 1, 1, 0.08); border.width: 1
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.topMargin: root.showing ? 54 : 34
        anchors.rightMargin: 10
        clip: true

        opacity: root.showing ? 1.0 : 0.0

        Behavior on anchors.topMargin {
            NumberAnimation { id: _slideAnim; duration: 250; easing.type: Easing.OutCubic }
        }
        Behavior on opacity {
            NumberAnimation { id: _opacityAnim; duration: 250; easing.type: Easing.OutCubic }
        }

        MouseArea { anchors.fill: parent; onClicked: mouse => mouse.accepted = true }

        ColumnLayout {
            id: col
            anchors.fill: parent; anchors.margins: 16; spacing: 8

            // 标题栏
            RowLayout {
                Layout.fillWidth: true

                Text {
                    text: "󰅍 剪贴板"
                    font.family: "Hack Nerd Font"; font.pixelSize: 15; font.bold: true
                    color: Colors.text
                }
                Item { Layout.fillWidth: true }
                Text {
                    visible: filteredModel.count > 0
                    text: filteredModel.count + " 条"
                    color: Colors.subtext0; font.family: "Hack Nerd Font"; font.pixelSize: 11
                }
                Rectangle {
                    visible: clipModel.count > 0
                    width: clearText.implicitWidth + 16; height: 26; radius: 13
                    color: clearArea.containsMouse ? Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.15) : "transparent"
                    Behavior on color { ColorAnimation { duration: 150 } }
                    Text {
                        id: clearText; anchors.centerIn: parent
                        text: "清空"; color: clearArea.containsMouse ? Colors.red : Colors.subtext0
                    Behavior on color { ColorAnimation { duration: 150 } }
                        font.family: "Hack Nerd Font"; font.pixelSize: 11
                    }
                    MouseArea {
                        id: clearArea; anchors.fill: parent; hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: wipeProc.running = true
                    }
                }
            }

            // 搜索框
            Rectangle {
                Layout.fillWidth: true; height: 36; radius: 10
                color: Colors.surface1

                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 10; spacing: 8
                    Text { text: ""; color: Colors.overlay1; font.family: "Hack Nerd Font"; font.pixelSize: 14 }
                    TextInput {
                        id: searchInput
                        Layout.fillWidth: true
                        color: Colors.text; font.family: "Hack Nerd Font"; font.pixelSize: 13
                        clip: true; selectByMouse: true
                        onTextChanged: { root.searchQuery = text; root.applyFilter() }
                        Keys.onEscapePressed: PanelState.clipboardOpen = false
                        Text {
                            anchors.fill: parent; text: "搜索..."
                            color: Colors.overlay0; font: parent.font
                            visible: !parent.text && !parent.activeFocus
                        }
                    }
                }
            }

            Rectangle { Layout.fillWidth: true; height: 1; color: Colors.surface1 }

            // 空状态
            Text {
                visible: filteredModel.count === 0
                text: clipModel.count === 0 ? "剪贴板为空" : "未找到匹配项"
                color: Colors.overlay0; font.family: "Hack Nerd Font"; font.pixelSize: 13
                Layout.alignment: Qt.AlignHCenter; Layout.topMargin: 20; Layout.bottomMargin: 20
            }

            // 列表
            ListView {
                Layout.fillWidth: true; Layout.fillHeight: true
                model: filteredModel; spacing: 4; clip: true

                delegate: Rectangle {
                    required property int index
                    required property string clipId
                    required property string preview

                    width: ListView.view.width; height: 40; radius: 8
                    color: itemHover.containsMouse ? Colors.surface1 : "transparent"
                    Behavior on color { ColorAnimation { duration: 100 } }

                    MouseArea {
                        id: itemHover; anchors.fill: parent
                        hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                        acceptedButtons: Qt.LeftButton
                        onClicked: root.selectItem(clipId)
                    }

                    RowLayout {
                        anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 6; spacing: 8

                        Text {
                            text: preview
                            color: Colors.text; font.family: "Hack Nerd Font"; font.pixelSize: 12
                            elide: Text.ElideRight; Layout.fillWidth: true; maximumLineCount: 1
                        }

                        // 删除按钮
                        Rectangle {
                            Layout.preferredWidth: 28; Layout.preferredHeight: 28
                            radius: 14
                            color: delArea.containsMouse ? Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.15) : "transparent"
                            Behavior on color { ColorAnimation { duration: 150 } }

                            Text {
                                anchors.centerIn: parent
                                text: "󰅖"
                                color: delArea.containsMouse ? Colors.red : Colors.overlay0
                                font.family: "Hack Nerd Font"; font.pixelSize: 14
                                Behavior on color { ColorAnimation { duration: 150 } }
                            }
                            MouseArea {
                                id: delArea; anchors.fill: parent
                                hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                onClicked: root.deleteItem(clipId)
                            }
                        }
                    }
                }
            }
        }
    }
}
