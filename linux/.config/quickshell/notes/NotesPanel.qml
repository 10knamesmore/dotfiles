import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

// 便签面板 — 右侧滑出，JSON 持久化
PanelWindow {
    id: root

    property bool showing: PanelState.notesOpen
    property bool animating: _opacityAnim.running || _slideAnim.running
    property string searchQuery: ""
    property int editingIndex: -1 // -1=列表模式，>=0=编辑模式
    property int nextId: 1

    readonly property string dataPath: (Qt.resolvedUrl("").toString().replace("file://", "").replace(/\/notes\/$/, "") + "/../../../.cache/quickshell/notes.json").replace("/../../../", "/home/" + _userProc._user + "/.cache/quickshell/notes.json")
    readonly property string _storePath: "/home/" + _userProc._user + "/.cache/quickshell/notes.json"

    property var colorOptions: [
        {"name": "blue", "color": Colors.blue},
        {"name": "mauve", "color": Colors.mauve},
        {"name": "green", "color": Colors.green},
        {"name": "peach", "color": Colors.peach},
        {"name": "red", "color": Colors.red},
        {"name": "yellow", "color": Colors.yellow}
    ]

    function noteColor(name) {
        for (let c of colorOptions) {
            if (c.name === name) return c.color;
        }
        return Colors.blue;
    }

    function loadNotes() {
        noteModel.clear();
        filteredModel.clear();
        loadProc.running = true;
    }

    function saveNotes() {
        let arr = [];
        for (let i = 0; i < noteModel.count; i++) {
            let item = noteModel.get(i);
            arr.push({
                id: item.noteId,
                text: item.text,
                color: item.noteColor,
                created: item.created
            });
        }
        let json = JSON.stringify(arr);
        saveProc.command = ["sh", "-c", "mkdir -p ~/.cache/quickshell && printf '%s' '" +
            json.replace(/'/g, "'\\''") + "' > " + root._storePath];
        saveProc.running = true;
    }

    function applyFilter() {
        filteredModel.clear();
        let q = searchQuery.toLowerCase();
        for (let i = 0; i < noteModel.count; i++) {
            let item = noteModel.get(i);
            if (q.length === 0 || item.text.toLowerCase().includes(q))
                filteredModel.append({
                    "noteId": item.noteId,
                    "text": item.text,
                    "noteColor": item.noteColor,
                    "created": item.created,
                    "sourceIndex": i
                });
        }
    }

    function addNote() {
        let id = nextId++;
        let now = new Date().toISOString().substring(0, 10);
        noteModel.insert(0, {
            "noteId": id,
            "text": "",
            "noteColor": "blue",
            "created": now
        });
        applyFilter();
        editingIndex = 0;
        saveNotes();
    }

    function deleteNote(idx) {
        // 找到 sourceIndex 并删除
        let item = filteredModel.get(idx);
        let srcIdx = item.sourceIndex;
        noteModel.remove(srcIdx);
        if (editingIndex === idx)
            editingIndex = -1;
        applyFilter();
        saveNotes();
    }

    function updateNoteText(idx, newText) {
        let item = filteredModel.get(idx);
        noteModel.setProperty(item.sourceIndex, "text", newText);
        filteredModel.setProperty(idx, "text", newText);
        saveDebounce.restart();
    }

    function updateNoteColor(idx, newColor) {
        let item = filteredModel.get(idx);
        noteModel.setProperty(item.sourceIndex, "noteColor", newColor);
        filteredModel.setProperty(idx, "noteColor", newColor);
        saveNotes();
    }

    anchors.top: true
    anchors.bottom: true
    anchors.left: true
    anchors.right: true
    visible: showing || animating
    focusable: root.showing
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"
    onShowingChanged: {
        if (showing) {
            searchQuery = "";
            editingIndex = -1;
            loadNotes();
        }
    }

    ListModel { id: noteModel }
    ListModel { id: filteredModel }

    Timer {
        id: saveDebounce
        interval: 800
        onTriggered: root.saveNotes()
    }

    // ── 获取用户名 ──
    Process {
        id: _userProc
        property string _user: "wanger"
        command: ["whoami"]
        Component.onCompleted: running = true
        stdout: SplitParser {
            onRead: data => _userProc._user = data.trim()
        }
    }

    // ── 加载 ──
    Process {
        id: loadProc
        command: ["sh", "-c", "cat " + root._storePath + " 2>/dev/null || echo '[]'"]
        stdout: SplitParser {
            onRead: data => {
                try {
                    let arr = JSON.parse(data);
                    let maxId = 0;
                    for (let item of arr) {
                        noteModel.append({
                            "noteId": item.id || 0,
                            "text": item.text || "",
                            "noteColor": item.color || "blue",
                            "created": item.created || ""
                        });
                        if (item.id > maxId) maxId = item.id;
                    }
                    root.nextId = maxId + 1;
                    root.applyFilter();
                } catch(e) {}
            }
        }
    }

    Process { id: saveProc }

    // ── UI ──
    Rectangle {
        anchors.fill: parent
        color: "#000000"
        opacity: root.showing ? Tokens.backdropDim : 0

        Behavior on opacity {
            NumberAnimation {
                duration: Tokens.animNormal
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Anim.standard
            }
        }
    }

    Item {
        focus: root.showing
        Keys.onEscapePressed: {
            if (root.editingIndex >= 0)
                root.editingIndex = -1;
            else
                PanelState.notesOpen = false;
        }
    }

    MouseArea {
        anchors.fill: parent
        onClicked: PanelState.notesOpen = false
    }

    Rectangle {
        id: panel

        width: 520
        height: root.height * 0.7
        radius: Tokens.radiusL
        color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, Tokens.panelAlpha)
        border.color: Qt.rgba(1, 1, 1, Tokens.borderAlpha)
        border.width: 1
        anchors.centerIn: parent
        anchors.verticalCenterOffset: root.showing ? 0 : 20
        clip: true
        opacity: root.showing ? 1 : 0

        SoftShadow {
            anchors.fill: parent
            radius: parent.radius
        }

        MouseArea {
            anchors.fill: parent
            onClicked: mouse => mouse.accepted = true
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: Tokens.spaceL
            spacing: Tokens.spaceS

            // ── 标题栏 ──
            RowLayout {
                Layout.fillWidth: true

                Text {
                    text: "󰎚 便签"
                    font.family: Fonts.family
                    font.pixelSize: Fonts.title
                    font.bold: true
                    color: Colors.text
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: noteModel.count + " 条"
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
                }

                // 新建按钮
                Rectangle {
                    width: addText.implicitWidth + 16
                    height: 26
                    radius: Tokens.radiusFull
                    color: addArea.containsMouse ? Qt.rgba(Colors.green.r, Colors.green.g, Colors.green.b, 0.15) : "transparent"

                    Text {
                        id: addText
                        anchors.centerIn: parent
                        text: "新建"
                        color: addArea.containsMouse ? Colors.green : Colors.subtext0
                        font.family: Fonts.family
                        font.pixelSize: Fonts.small

                        Behavior on color { ColorAnimation { duration: Tokens.animFast } }
                    }

                    MouseArea {
                        id: addArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.addNote()
                    }

                    Behavior on color { ColorAnimation { duration: Tokens.animFast } }
                }
            }

            // ── 搜索框 ──
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
                        Keys.onEscapePressed: PanelState.notesOpen = false

                        Text {
                            anchors.fill: parent
                            text: "搜索便签..."
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

            // ── 空状态 ──
            Text {
                visible: filteredModel.count === 0
                text: noteModel.count === 0 ? "还没有便签" : "未找到匹配项"
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.bodyLarge
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: 20
                Layout.bottomMargin: 20
            }

            // ── 便签列表 ──
            ListView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                model: filteredModel
                spacing: Tokens.spaceS
                clip: true

                delegate: Rectangle {
                    required property int index
                    required property int noteId
                    required property string text
                    required property string noteColor
                    required property string created

                    property bool isEditing: root.editingIndex === index

                    width: ListView.view.width
                    height: isEditing ? editCol.implicitHeight + 16 : 52
                    radius: Tokens.radiusMS
                    color: itemHover.containsMouse || isEditing ? Colors.surface1 : Colors.surface0
                    border.color: Qt.rgba(root.noteColor(noteColor).r, root.noteColor(noteColor).g, root.noteColor(noteColor).b, 0.3)
                    border.width: 1

                    Behavior on height {
                        NumberAnimation { duration: Tokens.animNormal; easing.type: Easing.OutCubic }
                    }

                    MouseArea {
                        id: itemHover
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (!isEditing)
                                root.editingIndex = index;
                        }
                    }

                    // ── 列表模式 ──
                    RowLayout {
                        visible: !isEditing
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: Tokens.spaceS

                        // 颜色指示条
                        Rectangle {
                            Layout.preferredWidth: 3
                            Layout.fillHeight: true
                            radius: 2
                            color: root.noteColor(noteColor)
                        }

                        // 预览
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2

                            Text {
                                text: (parent.parent.parent.text || "空便签").split("\n")[0]
                                color: Colors.text
                                font.family: Fonts.family
                                font.pixelSize: Fonts.body
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                                maximumLineCount: 1
                            }

                            Text {
                                text: created
                                color: Colors.overlay0
                                font.family: Fonts.family
                                font.pixelSize: Fonts.xs
                            }
                        }

                        // 删除
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

                                Behavior on color { ColorAnimation { duration: Tokens.animFast } }
                            }

                            MouseArea {
                                id: delArea
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.deleteNote(index)
                            }

                            Behavior on color { ColorAnimation { duration: Tokens.animFast } }
                        }
                    }

                    // ── 编辑模式 ──
                    ColumnLayout {
                        id: editCol
                        visible: isEditing
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: 10
                        spacing: Tokens.spaceS

                        // 颜色选择
                        Row {
                            spacing: 6

                            Repeater {
                                model: root.colorOptions

                                delegate: Rectangle {
                                    required property var modelData

                                    width: 18
                                    height: 18
                                    radius: 9
                                    color: modelData.color
                                    border.color: noteColor === modelData.name ? Colors.text : "transparent"
                                    border.width: 2

                                    MouseArea {
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: root.updateNoteColor(index, modelData.name)
                                    }
                                }
                            }

                            Item { width: 20; height: 1 }

                            // 关闭编辑
                            Text {
                                text: "󰅁 返回"
                                color: Colors.subtext0
                                font.family: Fonts.family
                                font.pixelSize: Fonts.small

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: root.editingIndex = -1
                                }
                            }
                        }

                        // 文本编辑
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: Math.max(120, editArea.implicitHeight + 16)
                            radius: Tokens.radiusS
                            color: Colors.surface0

                            TextEdit {
                                id: editArea
                                anchors.fill: parent
                                anchors.margins: 8
                                color: Colors.text
                                font.family: Fonts.family
                                font.pixelSize: Fonts.body
                                wrapMode: TextEdit.WordWrap
                                selectByMouse: true
                                text: parent.parent.parent.text
                                onTextChanged: {
                                    if (isEditing)
                                        root.updateNoteText(index, editArea.text);
                                }

                                Component.onCompleted: {
                                    if (isEditing) forceActiveFocus();
                                }
                            }
                        }
                    }

                    Behavior on color {
                        ColorAnimation { duration: Tokens.animFast }
                    }
                }
            }
        }

        InnerGlow {}

        Behavior on anchors.verticalCenterOffset {
            NumberAnimation {
                id: _slideAnim
                duration: Tokens.animSlow
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Anim.decelerate
            }
        }

        Behavior on opacity {
            NumberAnimation {
                id: _opacityAnim
                duration: Tokens.animNormal
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Anim.standard
            }
        }
    }
}
