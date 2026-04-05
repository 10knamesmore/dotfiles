import "../components"
import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell.Io

// 便签 + 待办面板 — 居中弹出，JSON 持久化
PanelOverlay {
    id: root

    showing: PanelState.notesOpen
    panelWidth: 520
    panelHeight: root.height * 0.75

    onCloseRequested: {
        if (editingIndex >= 0)
            editingIndex = -1;
        else
            PanelState.notesOpen = false;
    }

    onShowingChanged: {
        if (showing) {
            searchQuery = "";
            editingIndex = -1;
            loadNotes();
        }
    }

    property string searchQuery: ""
    property int editingIndex: -1
    property int _editingNoteId: -1
    onEditingIndexChanged: {
        if (editingIndex >= 0 && editingIndex < filteredModel.count)
            _editingNoteId = filteredModel.get(editingIndex).noteId;
        else
            _editingNoteId = -1;
    }
    property int nextId: 1
    property int filterMode: 0 // 0=全部 1=便签 2=待办未完成 3=已完成

    readonly property string _storePath: "/home/" + _userProc._user + "/.cache/quickshell/notes.json"

    // 统计
    property int _todoTotal: 0
    property int _todoDone: 0

    function _updateStats() {
        let total = 0, done = 0;
        for (let i = 0; i < noteModel.count; i++) {
            let item = noteModel.get(i);
            if (item.isTodo) {
                total++;
                if (item.done) done++;
            }
        }
        _todoTotal = total;
        _todoDone = done;
    }

    property var colorOptions: [
        {"name": "blue", "color": Colors.blue},
        {"name": "mauve", "color": Colors.mauve},
        {"name": "green", "color": Colors.green},
        {"name": "peach", "color": Colors.peach},
        {"name": "red", "color": Colors.red},
        {"name": "yellow", "color": Colors.yellow}
    ]

    property var priorityOptions: [
        {"name": "none", "label": "无", "color": Colors.overlay0},
        {"name": "low", "label": "低", "color": Colors.blue},
        {"name": "mid", "label": "中", "color": Colors.yellow},
        {"name": "high", "label": "高", "color": Colors.red}
    ]

    function noteColor(name) {
        for (let c of colorOptions) {
            if (c.name === name) return c.color;
        }
        return Colors.blue;
    }

    function priorityColor(name) {
        for (let p of priorityOptions) {
            if (p.name === name) return p.color;
        }
        return Colors.overlay0;
    }

    function priorityLabel(name) {
        for (let p of priorityOptions) {
            if (p.name === name) return p.label;
        }
        return "";
    }

    function priorityWeight(name) {
        if (name === "high") return 3;
        if (name === "mid") return 2;
        if (name === "low") return 1;
        return 0;
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
                created: item.created,
                isTodo: item.isTodo,
                done: item.done,
                pinned: item.pinned,
                priority: item.priority
            });
        }
        let json = JSON.stringify(arr);
        saveProc.command = ["sh", "-c", "mkdir -p ~/.cache/quickshell && printf '%s' '" +
            json.replace(/'/g, "'\\''") + "' > " + root._storePath];
        saveProc.running = true;
    }

    function applyFilter() {
        // 保存当前编辑条目的 noteId（在 clear 之前）
        let savedNoteId = _editingNoteId;
        filteredModel.clear();
        let q = searchQuery.toLowerCase();
        let items = [];
        for (let i = 0; i < noteModel.count; i++) {
            let item = noteModel.get(i);
            if (q.length > 0 && !item.text.toLowerCase().includes(q))
                continue;
            if (filterMode === 1 && item.isTodo) continue;
            if (filterMode === 2 && (!item.isTodo || item.done)) continue;
            if (filterMode === 3 && (!item.isTodo || !item.done)) continue;
            items.push({
                "noteId": item.noteId,
                "text": item.text,
                "noteColor": item.noteColor,
                "created": item.created,
                "isTodo": item.isTodo,
                "done": item.done,
                "pinned": item.pinned,
                "priority": item.priority,
                "sourceIndex": i
            });
        }
        // 排序：置顶优先，然后高优先级优先
        items.sort((a, b) => {
            if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
            let pa = priorityWeight(a.priority), pb = priorityWeight(b.priority);
            if (pa !== pb) return pb - pa;
            return 0; // 保持原始顺序
        });
        for (let item of items)
            filteredModel.append(item);
        // 恢复编辑索引
        if (savedNoteId >= 0) {
            let found = -1;
            for (let i = 0; i < filteredModel.count; i++) {
                if (filteredModel.get(i).noteId === savedNoteId) {
                    found = i;
                    break;
                }
            }
            editingIndex = found;
        }
        _updateStats();
    }

    function addNote(asTodo) {
        let id = nextId++;
        let now = new Date().toISOString().substring(0, 10);
        noteModel.insert(0, {
            "noteId": id,
            "text": "",
            "noteColor": asTodo ? "green" : "blue",
            "created": now,
            "isTodo": asTodo,
            "done": false,
            "pinned": false,
            "priority": "none"
        });
        applyFilter();
        // 找到新插入的条目在 filteredModel 中的位置
        for (let i = 0; i < filteredModel.count; i++) {
            if (filteredModel.get(i).noteId === id) {
                editingIndex = i;
                break;
            }
        }
        saveNotes();
    }

    function deleteNote(idx) {
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

    function toggleDone(idx) {
        let item = filteredModel.get(idx);
        let newVal = !item.done;
        noteModel.setProperty(item.sourceIndex, "done", newVal);
        filteredModel.setProperty(idx, "done", newVal);
        _updateStats();
        saveNotes();
    }

    function toggleTodo(idx) {
        let item = filteredModel.get(idx);
        let newVal = !item.isTodo;
        noteModel.setProperty(item.sourceIndex, "isTodo", newVal);
        filteredModel.setProperty(idx, "isTodo", newVal);
        if (!newVal) {
            noteModel.setProperty(item.sourceIndex, "done", false);
            filteredModel.setProperty(idx, "done", false);
            noteModel.setProperty(item.sourceIndex, "priority", "none");
            filteredModel.setProperty(idx, "priority", "none");
        }
        _updateStats();
        saveNotes();
    }

    function togglePin(idx) {
        let item = filteredModel.get(idx);
        let newVal = !item.pinned;
        noteModel.setProperty(item.sourceIndex, "pinned", newVal);
        filteredModel.setProperty(idx, "pinned", newVal);
        saveNotes();
        applyFilter();
    }

    function setPriority(idx, pri) {
        let item = filteredModel.get(idx);
        noteModel.setProperty(item.sourceIndex, "priority", pri);
        filteredModel.setProperty(idx, "priority", pri);
        saveNotes();
    }

    function clearCompleted() {
        for (let i = noteModel.count - 1; i >= 0; i--) {
            if (noteModel.get(i).isTodo && noteModel.get(i).done)
                noteModel.remove(i);
        }
        editingIndex = -1;
        applyFilter();
        saveNotes();
    }

    ListModel { id: noteModel }
    ListModel { id: filteredModel }

    Timer {
        id: saveDebounce
        interval: 800
        onTriggered: root.saveNotes()
    }

    Process {
        id: _userProc
        property string _user: "wanger"
        command: ["whoami"]
        Component.onCompleted: running = true
        stdout: SplitParser {
            onRead: data => _userProc._user = data.trim()
        }
    }

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
                            "created": item.created || "",
                            "isTodo": item.isTodo || false,
                            "done": item.done || false,
                            "pinned": item.pinned || false,
                            "priority": item.priority || "none"
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

    // ── 面板内容 ──
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

            // 待办统计
            Text {
                visible: root._todoTotal > 0
                text: root._todoDone + "/" + root._todoTotal
                color: root._todoDone === root._todoTotal ? Colors.green : Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }

            Item { Layout.fillWidth: true }

            // 清除已完成
            Rectangle {
                visible: root._todoDone > 0
                width: clearDoneText.implicitWidth + 16
                height: 26
                radius: Tokens.radiusFull
                color: clearDoneArea.containsMouse ? Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.15) : "transparent"

                Text {
                    id: clearDoneText
                    anchors.centerIn: parent
                    text: "清除已完成"
                    color: clearDoneArea.containsMouse ? Colors.red : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.xs
                    Behavior on color { ColorAnimation { duration: Tokens.animFast } }
                }

                MouseArea {
                    id: clearDoneArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.clearCompleted()
                }

                Behavior on color { ColorAnimation { duration: Tokens.animFast } }
            }

            // 新建便签
            Rectangle {
                width: addNoteText.implicitWidth + 16
                height: 26
                radius: Tokens.radiusFull
                color: addNoteArea.containsMouse ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15) : "transparent"

                Text {
                    id: addNoteText
                    anchors.centerIn: parent
                    text: "+ 便签"
                    color: addNoteArea.containsMouse ? Colors.blue : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
                    Behavior on color { ColorAnimation { duration: Tokens.animFast } }
                }

                MouseArea {
                    id: addNoteArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.addNote(false)
                }

                Behavior on color { ColorAnimation { duration: Tokens.animFast } }
            }

            // 新建待办
            Rectangle {
                width: addTodoText.implicitWidth + 16
                height: 26
                radius: Tokens.radiusFull
                color: addTodoArea.containsMouse ? Qt.rgba(Colors.green.r, Colors.green.g, Colors.green.b, 0.15) : "transparent"

                Text {
                    id: addTodoText
                    anchors.centerIn: parent
                    text: "+ 待办"
                    color: addTodoArea.containsMouse ? Colors.green : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
                    Behavior on color { ColorAnimation { duration: Tokens.animFast } }
                }

                MouseArea {
                    id: addTodoArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.addNote(true)
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
                    Keys.onEscapePressed: {
                        if (root.editingIndex >= 0)
                            root.editingIndex = -1;
                        else
                            PanelState.notesOpen = false;
                    }

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

        // ── 过滤标签 ──
        Row {
            Layout.fillWidth: true
            spacing: 4

            component FilterTab: Rectangle {
                property string label: ""
                property int mode: 0
                property bool active: root.filterMode === mode

                width: tabLabel.implicitWidth + 16
                height: 24
                radius: Tokens.radiusFull
                color: active ? Qt.rgba(Colors.mauve.r, Colors.mauve.g, Colors.mauve.b, 0.2) : tabArea.containsMouse ? Colors.surface1 : "transparent"

                Text {
                    id: tabLabel
                    anchors.centerIn: parent
                    text: label
                    color: active ? Colors.mauve : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.xs
                    font.weight: active ? Font.Bold : Font.Normal
                }

                MouseArea {
                    id: tabArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root.filterMode = mode;
                        root.applyFilter();
                    }
                }

                Behavior on color { ColorAnimation { duration: Tokens.animFast } }
            }

            FilterTab { label: "全部"; mode: 0 }
            FilterTab { label: "便签"; mode: 1 }
            FilterTab { label: "待办"; mode: 2 }
            FilterTab { label: "已完成"; mode: 3 }
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

        // ── 列表 ──
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
                required property bool isTodo
                required property bool done
                required property bool pinned
                required property string priority

                property bool isEditing: root.editingIndex === index

                width: ListView.view.width
                height: isEditing ? editCol.implicitHeight + 16 : 52
                radius: Tokens.radiusMS
                color: itemHover.containsMouse || isEditing ? Colors.surface1 : Colors.surface0
                border.color: Qt.rgba(root.noteColor(noteColor).r, root.noteColor(noteColor).g, root.noteColor(noteColor).b, pinned ? 0.6 : 0.3)
                border.width: pinned ? 2 : 1

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

                    // 颜色条（便签）
                    Rectangle {
                        visible: !isTodo
                        Layout.preferredWidth: 3
                        Layout.fillHeight: true
                        radius: 2
                        color: root.noteColor(noteColor)
                    }

                    // TODO checkbox
                    Rectangle {
                        visible: isTodo
                        Layout.preferredWidth: 22
                        Layout.preferredHeight: 22
                        radius: 11
                        color: "transparent"
                        border.color: done ? Colors.green : Colors.overlay1
                        border.width: 2

                        Text {
                            anchors.centerIn: parent
                            text: "󰄬"
                            color: Colors.green
                            font.family: Fonts.family
                            font.pixelSize: Fonts.body
                            visible: done
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.toggleDone(index)
                        }

                        Behavior on border.color { ColorAnimation { duration: Tokens.animFast } }
                    }

                    // 预览
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        RowLayout {
                            spacing: 4

                            // 置顶标记
                            Text {
                                visible: pinned
                                text: "󰐃"
                                color: Colors.yellow
                                font.family: Fonts.family
                                font.pixelSize: Fonts.xs
                            }

                            // 优先级标记
                            Rectangle {
                                visible: isTodo && priority !== "none"
                                width: priText.implicitWidth + 8
                                height: 14
                                radius: 7
                                color: Qt.rgba(root.priorityColor(priority).r, root.priorityColor(priority).g, root.priorityColor(priority).b, 0.2)

                                Text {
                                    id: priText
                                    anchors.centerIn: parent
                                    text: root.priorityLabel(priority)
                                    color: root.priorityColor(priority)
                                    font.family: Fonts.family
                                    font.pixelSize: 9
                                    font.weight: Font.Bold
                                }
                            }

                            Text {
                                text: (parent.parent.parent.parent.text || "空便签").split("\n")[0]
                                color: done ? Colors.overlay0 : Colors.text
                                font.family: Fonts.family
                                font.pixelSize: Fonts.body
                                font.strikeout: done
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                                maximumLineCount: 1
                                opacity: done ? 0.6 : 1
                            }
                        }

                        Text {
                            text: created
                            color: Colors.overlay0
                            font.family: Fonts.family
                            font.pixelSize: Fonts.xs
                        }
                    }

                    // 置顶按钮
                    Rectangle {
                        Layout.preferredWidth: 28
                        Layout.preferredHeight: 28
                        radius: Tokens.radiusFull
                        color: pinArea.containsMouse ? Qt.rgba(Colors.yellow.r, Colors.yellow.g, Colors.yellow.b, 0.15) : "transparent"

                        Text {
                            anchors.centerIn: parent
                            text: pinned ? "󰐃" : "󰐄"
                            color: pinned ? Colors.yellow : (pinArea.containsMouse ? Colors.yellow : Colors.overlay0)
                            font.family: Fonts.family
                            font.pixelSize: Fonts.icon
                            Behavior on color { ColorAnimation { duration: Tokens.animFast } }
                        }

                        MouseArea {
                            id: pinArea
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.togglePin(index)
                        }

                        Behavior on color { ColorAnimation { duration: Tokens.animFast } }
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

                    // 工具行
                    Row {
                        spacing: 6

                        // 便签/待办切换
                        Rectangle {
                            width: typeLabel.implicitWidth + 14
                            height: 22
                            radius: Tokens.radiusFull
                            color: isTodo
                                ? Qt.rgba(Colors.green.r, Colors.green.g, Colors.green.b, 0.2)
                                : Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.2)

                            Text {
                                id: typeLabel
                                anchors.centerIn: parent
                                text: isTodo ? "待办" : "便签"
                                color: isTodo ? Colors.green : Colors.blue
                                font.family: Fonts.family
                                font.pixelSize: Fonts.xs
                                font.weight: Font.Bold
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.toggleTodo(index)
                            }
                        }

                        // done 状态（仅 todo）
                        Rectangle {
                            visible: isTodo
                            width: doneLabel.implicitWidth + 14
                            height: 22
                            radius: Tokens.radiusFull
                            color: done
                                ? Qt.rgba(Colors.green.r, Colors.green.g, Colors.green.b, 0.15)
                                : Qt.rgba(1, 1, 1, 0.06)

                            Text {
                                id: doneLabel
                                anchors.centerIn: parent
                                text: done ? "󰄬 已完成" : "未完成"
                                color: done ? Colors.green : Colors.overlay1
                                font.family: Fonts.family
                                font.pixelSize: Fonts.xs
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.toggleDone(index)
                            }
                        }

                        // 优先级选择（仅 todo）
                        Repeater {
                            model: isTodo ? root.priorityOptions : []

                            delegate: Rectangle {
                                required property var modelData
                                property bool active: priority === modelData.name

                                width: 22
                                height: 22
                                radius: Tokens.radiusFull
                                color: active ? Qt.rgba(modelData.color.r, modelData.color.g, modelData.color.b, 0.2) : priMouseArea.containsMouse ? Colors.surface2 : "transparent"

                                Text {
                                    anchors.centerIn: parent
                                    text: modelData.label
                                    color: active ? modelData.color : Colors.overlay0
                                    font.family: Fonts.family
                                    font.pixelSize: 9
                                    font.weight: active ? Font.Bold : Font.Normal
                                }

                                MouseArea {
                                    id: priMouseArea
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: root.setPriority(index, modelData.name)
                                }
                            }
                        }

                        Item { width: 4; height: 1 }

                        // 置顶
                        Rectangle {
                            width: 22; height: 22; radius: Tokens.radiusFull
                            color: pinned ? Qt.rgba(Colors.yellow.r, Colors.yellow.g, Colors.yellow.b, 0.2) : editPinArea.containsMouse ? Colors.surface2 : "transparent"

                            Text {
                                anchors.centerIn: parent
                                text: pinned ? "󰐃" : "󰐄"
                                color: pinned ? Colors.yellow : Colors.overlay0
                                font.family: Fonts.family
                                font.pixelSize: Fonts.small
                            }

                            MouseArea {
                                id: editPinArea
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.togglePin(index)
                            }
                        }

                        Item { width: 4; height: 1 }

                        // 颜色选择
                        Repeater {
                            model: root.colorOptions

                            delegate: Rectangle {
                                required property var modelData

                                width: 16
                                height: 16
                                radius: 8
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

                        Item { Layout.fillWidth: true; width: 1; height: 1 }

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
}
