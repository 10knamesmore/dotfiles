import "../components"
import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

// AI 聊天面板 — 右侧滑出，对接 AstrBot HTTP API
PanelOverlay {
    id: root

    showing: PanelState.aiOpen
    panelWidth: 600
    panelHeight: root.height * 0.85
    onCloseRequested: PanelState.aiOpen = false

    property bool generating: false
    property bool apiReachable: false
    property string _currentContent: ""
    property var _lastStats: null  // agent_stats: {token_usage, start_time, end_time, time_to_first_token}

    // ── 持久化状态 ──
    property string _server: ""
    property string _apiKey: ""
    property string _username: "qml-user"
    property string _currentSessionId: ""
    property string _currentConfigId: ""
    property string _currentConfigName: ""

    // ── 会话/配置列表 ──
    property var _configs: []
    property var _sessions: []   // 本地会话索引 [{session_id, title, updated_at}]

    // ── UI 状态 ──
    property bool showSessionDrawer: false
    property bool showConfigPicker: false
    property bool showSettings: false
    property bool showToolMessages: false  // 是否渲染 tool call/result 气泡
    property bool _initialized: false   // state 文件是否已加载过

    readonly property string _stateDir: "/home/" + _userProc._user + "/.cache/quickshell"
    readonly property string _statePath: _stateDir + "/ai_state.json"
    readonly property string _sessionsPath: _stateDir + "/ai_sessions.json"
    readonly property string _messagesDir: _stateDir + "/ai_messages"

    // ── 初始化 ──
    function init() {
        loadStateProc.running = true;
    }

    function generateSessionId() {
        return Date.now().toString(36) + Math.random().toString(36).substring(2, 8);
    }

    // ── 发送消息 ──
    function sendMessage() {
        let text = inputEdit.text.trim();
        if (text.length === 0 || !apiReachable || generating)
            return;

        messageModel.append({ role: "user", content: text });

        generating = true;
        _currentContent = "";
        messageModel.append({ role: "assistant", content: "" });

        // session_id 由服务端分配（通过 chatSessionId 信号回传）
        // 已有 session 则传入，新会话传空让服务端生成
        api.sendChat(text, _currentSessionId, _currentConfigId, _username);
        inputEdit.text = "";
        scrollToBottom();
    }

    // ── 新建会话 ──
    function newSession() {
        if (generating)
            return;
        // 保存当前会话消息
        if (_currentSessionId && messageModel.count > 0)
            saveMessages();

        _currentSessionId = "";
        messageModel.clear();
        _currentContent = "";
        saveState();
    }

    // ── 切换会话 ──
    function switchSession(sessionId) {
        if (generating || sessionId === _currentSessionId)
            return;
        // 保存当前
        if (_currentSessionId && messageModel.count > 0)
            saveMessages();

        _currentSessionId = sessionId;
        messageModel.clear();
        _currentContent = "";
        loadMessages(sessionId);
        saveState();
    }

    // ── 删除会话 ──
    function deleteSession(sessionId) {
        let arr = [];
        for (let i = 0; i < _sessions.length; i++) {
            if (_sessions[i].session_id !== sessionId)
                arr.push(_sessions[i]);
        }
        _sessions = arr;
        saveSessionsIndex();

        // 删除消息文件
        delMsgProc.command = ["rm", "-f", _messagesDir + "/" + sessionId + ".json"];
        delMsgProc.running = true;

        // 如果删的是当前会话
        if (sessionId === _currentSessionId) {
            _currentSessionId = "";
            messageModel.clear();
            saveState();
        }
    }

    // ── 清空当前对话 ──
    function clearChat() {
        if (_currentSessionId)
            deleteSession(_currentSessionId);
        else
            messageModel.clear();
        generating = false;
        _currentContent = "";
    }

    function scrollToBottom() {
        scrollTimer.start();
    }

    // ── 持久化: State ──
    function saveState() {
        let obj = {
            server: _server,
            apiKey: _apiKey,
            username: _username,
            currentSessionId: _currentSessionId,
            currentConfigId: _currentConfigId,
            currentConfigName: _currentConfigName,
            showToolMessages: showToolMessages
        };
        let json = JSON.stringify(obj);
        saveStateProc.command = ["sh", "-c", "mkdir -p " + _stateDir + " && printf '%s' '" +
            json.replace(/'/g, "'\\''") + "' > " + _statePath];
        saveStateProc.running = true;
    }

    // ── 持久化: Sessions Index ──
    function saveSessionsIndex() {
        let json = JSON.stringify(_sessions);
        saveSessionsProc.command = ["sh", "-c", "mkdir -p " + _stateDir + " && printf '%s' '" +
            json.replace(/'/g, "'\\''") + "' > " + _sessionsPath];
        saveSessionsProc.running = true;
    }

    // ── 持久化: Messages ──
    function saveMessages() {
        if (!_currentSessionId)
            return;
        let arr = [];
        for (let i = 0; i < messageModel.count; i++) {
            let item = messageModel.get(i);
            if (item.role !== "error" && item.content.trim())
                arr.push({ role: item.role, content: item.content, ts: Date.now() });
        }
        let json = JSON.stringify(arr);
        let sid = _currentSessionId;
        saveMsgProc.command = ["sh", "-c", "mkdir -p " + _messagesDir + " && printf '%s' '" +
            json.replace(/'/g, "'\\''") + "' > " + _messagesDir + "/" + sid + ".json"];
        saveMsgProc.running = true;

        // 更新会话索引的 updated_at
        for (let i = 0; i < _sessions.length; i++) {
            if (_sessions[i].session_id === sid) {
                _sessions[i].updated_at = Date.now();
                break;
            }
        }
    }

    function loadMessages(sessionId) {
        loadMsgProc._targetSession = sessionId;
        loadMsgProc.command = ["sh", "-c", "cat " + _messagesDir + "/" + sessionId + ".json 2>/dev/null || echo '[]'"];
        loadMsgProc.running = true;
    }

    onShowingChanged: {
        if (showing) {
            focusTimer.start();
            if (!_initialized && _userProc._user) {
                // whoami 已完成，加载持久化状态
                init();
            } else if (_initialized && _server) {
                api.fetchConfigs();
            }
            // 如果 whoami 还没完成，onExited 回调会触发 init
        }
    }

    ListModel {
        id: messageModel
    }

    Timer {
        id: scrollTimer
        property int _retries: 0
        interval: 50
        onTriggered: {
            if (msgView.count > 0)
                msgView.positionViewAtEnd();
            // 初次加载时多试几次确保布局完成
            if (_retries < 3) {
                _retries++;
                start();
            } else {
                _retries = 0;
            }
        }
    }

    Timer {
        id: focusTimer
        interval: 50
        onTriggered: inputEdit.forceActiveFocus()
    }

    Timer {
        id: saveMsgDebounce
        interval: 800
        onTriggered: root.saveMessages()
    }

    // ── 获取用户名 ──
    Process {
        id: _userProc
        property string _user: ""
        command: ["whoami"]
        running: true
        stdout: SplitParser {
            onRead: data => _userProc._user = data.trim()
        }
        onExited: {
            // whoami 完成后，如果面板已打开且未初始化，立即加载 state
            if (root.showing && !root._initialized)
                root.init();
        }
    }

    // ── AstrBot API ──
    AstrBotApi {
        id: api
        server: root._server
        apiKey: root._apiKey
    }

    Connections {
        target: api
        function onChatSessionId(sessionId) {
            // 服务端分配的 session_id，用于后续请求
            if (!root._currentSessionId || root._currentSessionId !== sessionId) {
                // 更新本地索引中的 session_id（如果是客户端生成的临时 ID）
                let oldId = root._currentSessionId;
                root._currentSessionId = sessionId;
                // 更新会话索引
                let found = false;
                for (let i = 0; i < root._sessions.length; i++) {
                    if (root._sessions[i].session_id === oldId) {
                        root._sessions[i].session_id = sessionId;
                        found = true;
                        break;
                    }
                }
                if (!found && messageModel.count > 0) {
                    let firstMsg = messageModel.get(0);
                    let title = firstMsg.content;
                    if (title.length > 20) title = title.substring(0, 20) + "…";
                    root._sessions = [{ session_id: sessionId, title: title, updated_at: Date.now() }].concat(root._sessions);
                }
                root.saveSessionsIndex();
                root.saveState();
            }
        }
        function onChatChunk(content) {
            // 如果最后一条不是 assistant，先追加一条空的
            if (messageModel.count === 0 || messageModel.get(messageModel.count - 1).role !== "assistant") {
                root._currentContent = "";
                messageModel.append({ role: "assistant", content: "" });
            }
            root._currentContent += content;
            // 只有有实际内容时才更新显示
            let trimmed = root._currentContent.trim();
            if (trimmed)
                messageModel.setProperty(messageModel.count - 1, "content", root._currentContent);
            root.scrollToBottom();
        }
        function onChatToolCall(name, args) {
            root._currentContent = "";
            messageModel.append({ role: "tool", content: "⚙ " + name + "\n" + args });
            root.scrollToBottom();
        }
        function onChatToolResult(name, result) {
            root._currentContent = "";
            messageModel.append({ role: "tool_result", content: "↳ " + (name || "result") + "\n" + result });
            root.scrollToBottom();
        }
        function onChatComplete(fullContent) {
            root.generating = false;
            root._currentContent = "";
            // 移除末尾空 assistant 消息（tool call 后可能残留）
            while (messageModel.count > 0) {
                let last = messageModel.get(messageModel.count - 1);
                if (last.role === "assistant" && !last.content)
                    messageModel.remove(messageModel.count - 1);
                else
                    break;
            }
            saveMsgDebounce.restart();
            // 面板未显示时发通知
            if (!root.showing) {
                let preview = fullContent.length > 60 ? fullContent.substring(0, 60) + "…" : fullContent;
                notifyProc.command = ["notify-send", "-a", "AI 助手", "AI 回复完成", preview || "消息已生成"];
                notifyProc.running = true;
            }
        }
        function onChatError(message) {
            root.generating = false;
            root._currentContent = "";
            messageModel.append({ role: "error", content: message });
            root.scrollToBottom();
        }
        function onChatStats(stats) {
            root._lastStats = stats;
        }
        function onConfigsLoaded(configs) {
            root._configs = configs;
            root.apiReachable = true;
            if (!root._currentConfigId && configs.length > 0) {
                for (let c of configs) {
                    if (c.is_default) {
                        root._currentConfigId = c.id;
                        root._currentConfigName = c.name;
                        break;
                    }
                }
                if (!root._currentConfigId) {
                    root._currentConfigId = configs[0].id;
                    root._currentConfigName = configs[0].name;
                }
            }
        }
        function onSessionsLoaded(sessions) {
            // AstrBot 的会话列表仅供参考，不覆盖本地索引
        }
    }

    // ── 持久化 Process ──
    Process {
        id: loadStateProc
        property string _buf: ""
        command: ["sh", "-c", "cat " + root._statePath + " 2>/dev/null || echo '{}'"]
        stdout: SplitParser {
            onRead: data => loadStateProc._buf += data
        }
        onExited: {
            try {
                let obj = JSON.parse(_buf);
                if (obj.server) root._server = obj.server;
                if (obj.apiKey) root._apiKey = obj.apiKey;
                if (obj.username) root._username = obj.username;
                if (obj.currentSessionId) root._currentSessionId = obj.currentSessionId;
                if (obj.currentConfigId) root._currentConfigId = obj.currentConfigId;
                if (obj.currentConfigName) root._currentConfigName = obj.currentConfigName;
                if (obj.showToolMessages !== undefined) root.showToolMessages = obj.showToolMessages;
            } catch (e) {}
            _buf = "";
            root._initialized = true;

            // 加载会话索引
            loadSessionsProc.running = true;

            // 获取配置（也作为连通性检测），无 server 则弹设置
            if (root._server)
                api.fetchConfigs();
            else
                root.showSettings = true;
        }
    }

    Process {
        id: loadSessionsProc
        property string _buf: ""
        command: ["sh", "-c", "cat " + root._sessionsPath + " 2>/dev/null || echo '[]'"]
        stdout: SplitParser {
            onRead: data => loadSessionsProc._buf += data
        }
        onExited: {
            try {
                root._sessions = JSON.parse(_buf);
            } catch (e) {
                root._sessions = [];
            }
            _buf = "";

            // 加载当前会话消息
            if (root._currentSessionId)
                root.loadMessages(root._currentSessionId);
        }
    }

    Process {
        id: loadMsgProc
        property string _buf: ""
        property string _targetSession: ""
        stdout: SplitParser {
            onRead: data => loadMsgProc._buf += data
        }
        onExited: {
            // 确保加载的还是当前会话
            if (_targetSession === root._currentSessionId) {
                try {
                    let arr = JSON.parse(_buf);
                    for (let msg of arr)
                        messageModel.append({ role: msg.role, content: msg.content });
                    root.scrollToBottom();
                } catch (e) {}
            }
            _buf = "";
        }
    }

    Process { id: saveStateProc }
    Process { id: saveSessionsProc }
    Process { id: saveMsgProc }
    Process { id: delMsgProc }
    Process { id: copyProc }
    Process { id: notifyProc }

    // ── UI ──
    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Tokens.spaceL
        spacing: Tokens.spaceS

        // ── 标题栏 ──
        RowLayout {
            Layout.fillWidth: true
            spacing: Tokens.spaceS

            Text {
                text: "󰧑 AI 助手"
                font.family: Fonts.family
                font.pixelSize: Fonts.title
                font.bold: true
                color: Colors.text
            }

            // 状态指示
            Rectangle {
                width: 8
                height: 8
                radius: 4
                color: root.apiReachable ? (root.generating ? Colors.yellow : Colors.green) : Colors.red

                Behavior on color {
                    ColorAnimation { duration: Tokens.animNormal }
                }
            }

            // 配置选择按钮
            Rectangle {
                width: configLabel.implicitWidth + 16
                height: 24
                radius: Tokens.radiusFull
                color: configArea.containsMouse ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15) : Qt.rgba(1, 1, 1, 0.05)

                Text {
                    id: configLabel
                    anchors.centerIn: parent
                    text: root._currentConfigName || "配置"
                    color: configArea.containsMouse ? Colors.blue : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.small
                    elide: Text.ElideRight

                    Behavior on color {
                        ColorAnimation { duration: Tokens.animFast }
                    }
                }

                MouseArea {
                    id: configArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.showConfigPicker = !root.showConfigPicker
                }

                Behavior on color {
                    ColorAnimation { duration: Tokens.animFast }
                }
            }

            // 工具调用开关
            Rectangle {
                width: toolRow.implicitWidth + 12
                height: 24
                radius: Tokens.radiusFull
                color: Qt.rgba(1, 1, 1, 0.05)

                RowLayout {
                    id: toolRow
                    anchors.centerIn: parent
                    spacing: 6

                    Text {
                        text: "工具"
                        color: Colors.subtext0
                        font.family: Fonts.family
                        font.pixelSize: Fonts.xs
                    }

                    ToggleSwitch {
                        small: true
                        checked: root.showToolMessages
                        onToggled: { root.showToolMessages = !root.showToolMessages; root.saveState(); }
                    }
                }
            }

            Item { Layout.fillWidth: true }

            // 会话历史按钮
            Rectangle {
                width: 28
                height: 28
                radius: Tokens.radiusFull
                color: historyArea.containsMouse ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15) : "transparent"

                Text {
                    anchors.centerIn: parent
                    text: "󰋚"
                    color: historyArea.containsMouse ? Colors.blue : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.iconLarge

                    Behavior on color {
                        ColorAnimation { duration: Tokens.animFast }
                    }
                }

                MouseArea {
                    id: historyArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.showSessionDrawer = !root.showSessionDrawer
                }

                Behavior on color {
                    ColorAnimation { duration: Tokens.animFast }
                }
            }

            // 新建会话按钮
            Rectangle {
                width: 28
                height: 28
                radius: Tokens.radiusFull
                color: newChatArea.containsMouse ? Qt.rgba(Colors.green.r, Colors.green.g, Colors.green.b, 0.15) : "transparent"

                Text {
                    anchors.centerIn: parent
                    text: "󰐕"
                    color: newChatArea.containsMouse ? Colors.green : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.iconLarge

                    Behavior on color {
                        ColorAnimation { duration: Tokens.animFast }
                    }
                }

                MouseArea {
                    id: newChatArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.newSession()
                }

                Behavior on color {
                    ColorAnimation { duration: Tokens.animFast }
                }
            }

            // 清空按钮
            Rectangle {
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
                        ColorAnimation { duration: Tokens.animFast }
                    }
                }

                MouseArea {
                    id: clearArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.clearChat()
                }

                Behavior on color {
                    ColorAnimation { duration: Tokens.animFast }
                }
            }

            // 设置按钮
            Rectangle {
                width: 28
                height: 28
                radius: Tokens.radiusFull
                color: settingsArea.containsMouse ? Qt.rgba(1, 1, 1, 0.08) : "transparent"

                Text {
                    anchors.centerIn: parent
                    text: "󰒓"
                    color: settingsArea.containsMouse ? Colors.text : Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.iconLarge

                    Behavior on color {
                        ColorAnimation { duration: Tokens.animFast }
                    }
                }

                MouseArea {
                    id: settingsArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.showSettings = !root.showSettings
                }

                Behavior on color {
                    ColorAnimation { duration: Tokens.animFast }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: Colors.surface1
        }

        // ── 消息列表 ──
        ListView {
            id: msgView

            Layout.fillWidth: true
            Layout.fillHeight: true
            model: messageModel
            spacing: Tokens.spaceS
            clip: true

            // 空状态
            Text {
                anchors.centerIn: parent
                visible: messageModel.count === 0
                text: root.apiReachable ? "发送消息开始对话" : (root._server ? "正在连接..." : "请先配置服务器地址")
                color: Colors.overlay0
                font.family: Fonts.family
                font.pixelSize: Fonts.bodyLarge
            }

            delegate: Item {
                required property int index
                required property string role
                required property string content

                width: msgView.width
                height: (bubble.isAnyTool && !root.showToolMessages) ? 0 : bubble.height
                visible: !(bubble.isAnyTool && !root.showToolMessages)

                HoverHandler {
                    id: bubbleHover
                }

                Rectangle {
                    id: bubble

                    property bool isUser: role === "user"
                    property bool isError: role === "error"
                    property bool isTool: role === "tool"
                    property bool isToolResult: role === "tool_result"
                    property bool isAnyTool: isTool || isToolResult

                    anchors.right: isUser ? parent.right : undefined
                    anchors.left: isUser ? undefined : parent.left
                    width: Math.min(msgText.implicitWidth + 24, parent.width * 0.85)
                    height: msgText.implicitHeight + 16
                    radius: Tokens.radiusMS

                    color: {
                        if (isUser)
                            return Qt.rgba(Colors.mauve.r, Colors.mauve.g, Colors.mauve.b, 0.2);
                        if (isError)
                            return Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.1);
                        if (isToolResult)
                            return Qt.rgba(Colors.green.r, Colors.green.g, Colors.green.b, 0.08);
                        if (isTool)
                            return Qt.rgba(Colors.peach.r, Colors.peach.g, Colors.peach.b, 0.08);
                        return Colors.surface0;
                    }

                    border.color: {
                        if (isUser)
                            return Qt.rgba(Colors.mauve.r, Colors.mauve.g, Colors.mauve.b, 0.15);
                        if (isError)
                            return Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.15);
                        if (isToolResult)
                            return Qt.rgba(Colors.green.r, Colors.green.g, Colors.green.b, 0.12);
                        if (isTool)
                            return Qt.rgba(Colors.peach.r, Colors.peach.g, Colors.peach.b, 0.12);
                        return Qt.rgba(1, 1, 1, 0.04);
                    }
                    border.width: 1

                    TextEdit {
                        id: msgText

                        anchors.fill: parent
                        anchors.margins: 12
                        anchors.rightMargin: 12
                        text: content || (role === "assistant" && root.generating && index === messageModel.count - 1 ? "..." : "")
                        color: {
                            if (bubble.isError) return Colors.red;
                            if (bubble.isToolResult) return Colors.green;
                            if (bubble.isTool) return Colors.peach;
                            return Colors.text;
                        }
                        font.family: Fonts.family
                        font.pixelSize: bubble.isAnyTool ? Fonts.body : Fonts.title
                        wrapMode: TextEdit.WordWrap
                        textFormat: (bubble.isUser || bubble.isAnyTool || bubble.isError) ? TextEdit.PlainText : TextEdit.MarkdownText
                        readOnly: true
                        selectByMouse: true
                        selectedTextColor: Colors.base
                        selectionColor: Colors.mauve
                    }

                }

                // 复制按钮（气泡外侧）
                Rectangle {
                    id: copyBtn
                    property bool copied: false

                    width: 26
                    height: 26
                    radius: Tokens.radiusFull
                    anchors.bottom: bubble.bottom
                    anchors.bottomMargin: 4
                    // user: 贴气泡左侧外边；assistant/tool: 贴气泡右侧外边
                    x: bubble.isUser ? (bubble.x - width - 4) : (bubble.x + bubble.width + 4)
                    visible: bubbleHover.hovered || copyArea.containsMouse || copied
                    color: copyArea.containsMouse ? Qt.rgba(1, 1, 1, 0.12) : "transparent"

                    Text {
                        anchors.centerIn: parent
                        text: copyBtn.copied ? "󰄬" : "󰆏"
                        color: copyBtn.copied ? Colors.green : (copyArea.containsMouse ? Colors.text : Colors.overlay1)
                        font.family: Fonts.family
                        font.pixelSize: Fonts.body

                        Behavior on color {
                            ColorAnimation { duration: Tokens.animFast }
                        }
                    }

                    MouseArea {
                        id: copyArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            copyProc.command = ["sh", "-c", "printf '%s' '" + content.replace(/'/g, "'\\''") + "' | wl-copy"];
                            copyProc.running = true;
                            copyBtn.copied = true;
                            copyResetTimer.start();
                        }
                    }

                    Timer {
                        id: copyResetTimer
                        interval: 1500
                        onTriggered: copyBtn.copied = false
                    }

                    Behavior on color {
                        ColorAnimation { duration: Tokens.animFast }
                    }
                }
            }
        }

        // ── Stats 显示 ──
        Text {
            Layout.fillWidth: true
            visible: root._lastStats !== null && !root.generating
            text: {
                if (!root._lastStats) return "";
                let s = root._lastStats;
                let parts = [];
                if (s.token_usage) {
                    let u = s.token_usage;
                    let input = (u.input_other || 0) + (u.input_cached || 0);
                    parts.push("tokens: " + input + "→" + (u.output || 0));
                    if (u.input_cached > 0)
                        parts.push("cached: " + u.input_cached);
                }
                if (s.time_to_first_token !== undefined)
                    parts.push("TTFT: " + s.time_to_first_token.toFixed(2) + "s");
                if (s.start_time && s.end_time)
                    parts.push("total: " + (s.end_time - s.start_time).toFixed(2) + "s");
                return parts.join("  ·  ");
            }
            color: Colors.overlay0
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            horizontalAlignment: Text.AlignHCenter
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: Colors.surface1
        }

        // ── 输入区域 ──
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(Math.max(inputEdit.implicitHeight + 16, 44), 120)
            radius: Tokens.radiusMS
            color: Colors.surface1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 8
                anchors.topMargin: 4
                anchors.bottomMargin: 4
                spacing: Tokens.spaceS

                TextEdit {
                    id: inputEdit

                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.body
                    wrapMode: TextEdit.Wrap
                    selectByMouse: true
                    verticalAlignment: TextEdit.AlignVCenter

                    Keys.onReturnPressed: event => {
                        if (event.modifiers & Qt.ShiftModifier) {
                            inputEdit.insert(inputEdit.cursorPosition, "\n");
                        } else {
                            root.sendMessage();
                            event.accepted = true;
                        }
                    }
                    Keys.onEscapePressed: PanelState.aiOpen = false

                    Text {
                        anchors.fill: parent
                        anchors.topMargin: 2
                        text: "输入消息..."
                        color: Colors.overlay0
                        font: parent.font
                        visible: !parent.text && !parent.activeFocus
                        verticalAlignment: Text.AlignVCenter
                    }
                }

                // 发送/停止按钮
                Rectangle {
                    Layout.preferredWidth: 32
                    Layout.preferredHeight: 32
                    Layout.alignment: Qt.AlignBottom
                    radius: Tokens.radiusFull
                    color: {
                        if (root.generating)
                            return sendArea.containsMouse ? Qt.rgba(Colors.red.r, Colors.red.g, Colors.red.b, 0.15) : "transparent";
                        return sendArea.containsMouse && root.apiReachable ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15) : "transparent";
                    }

                    Text {
                        anchors.centerIn: parent
                        text: root.generating ? "󰓛" : "󰒊"
                        color: {
                            if (root.generating)
                                return sendArea.containsMouse ? Colors.red : Colors.overlay1;
                            return root.apiReachable ? (sendArea.containsMouse ? Colors.blue : Colors.overlay1) : Colors.surface2;
                        }
                        font.family: Fonts.family
                        font.pixelSize: Fonts.iconLarge

                        Behavior on color {
                            ColorAnimation { duration: Tokens.animFast }
                        }
                    }

                    MouseArea {
                        id: sendArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: root.apiReachable || root.generating ? Qt.PointingHandCursor : Qt.ArrowCursor
                        onClicked: {
                            if (root.generating)
                                api.abortChat();
                            else
                                root.sendMessage();
                        }
                    }

                    Behavior on color {
                        ColorAnimation { duration: Tokens.animFast }
                    }
                }
            }
        }
    }

    // ── ConfigPicker 弹出层 ──
    ConfigPicker {
        id: configPicker
        visible: root.showConfigPicker
        configs: root._configs
        currentConfigId: root._currentConfigId
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.topMargin: 56
        anchors.leftMargin: Tokens.spaceL
        anchors.rightMargin: Tokens.spaceL

        onConfigSelected: (configId, configName) => {
            root._currentConfigId = configId;
            root._currentConfigName = configName;
            root.showConfigPicker = false;
            root.saveState();
        }
    }

    // ── SessionDrawer 覆盖层 ──
    SessionDrawer {
        id: sessionDrawer
        visible: root.showSessionDrawer
        sessions: root._sessions
        currentSessionId: root._currentSessionId
        anchors.fill: parent

        onSessionSelected: sessionId => {
            root.switchSession(sessionId);
            root.showSessionDrawer = false;
        }
        onNewSessionRequested: {
            root.newSession();
            root.showSessionDrawer = false;
        }
        onSessionDeleted: sessionId => {
            root.deleteSession(sessionId);
        }
        onCloseRequested: {
            root.showSessionDrawer = false;
        }
    }

    // ── 设置视图 ──
    Rectangle {
        id: settingsView
        visible: root.showSettings
        anchors.fill: parent
        radius: Tokens.radiusL
        color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, 0.98)
        z: 20

        MouseArea {
            anchors.fill: parent
            onClicked: mouse => mouse.accepted = true
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: Tokens.spaceL
            spacing: Tokens.spaceM

            RowLayout {
                Layout.fillWidth: true

                Text {
                    text: "󰒓 设置"
                    font.family: Fonts.family
                    font.pixelSize: Fonts.title
                    font.bold: true
                    color: Colors.text
                }

                Item { Layout.fillWidth: true }

                Rectangle {
                    width: 28
                    height: 28
                    radius: Tokens.radiusFull
                    color: settingsCloseArea.containsMouse ? Qt.rgba(1, 1, 1, 0.08) : "transparent"

                    Text {
                        anchors.centerIn: parent
                        text: "󰅖"
                        color: settingsCloseArea.containsMouse ? Colors.text : Colors.subtext0
                        font.family: Fonts.family
                        font.pixelSize: Fonts.iconLarge
                    }

                    MouseArea {
                        id: settingsCloseArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.showSettings = false
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: Colors.surface1
            }

            // 服务器地址
            Text {
                text: "服务器地址"
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }

            Rectangle {
                Layout.fillWidth: true
                height: 40
                radius: Tokens.radiusMS
                color: Colors.surface1

                TextEdit {
                    id: serverEdit
                    anchors.fill: parent
                    anchors.margins: 10
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.body
                    verticalAlignment: TextEdit.AlignVCenter
                    selectByMouse: true
                    text: root._server

                    Text {
                        anchors.fill: parent
                        text: "http://host:6185"
                        color: Colors.overlay0
                        font: parent.font
                        visible: !parent.text && !parent.activeFocus
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }

            // API Key
            Text {
                text: "API Key"
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }

            Rectangle {
                Layout.fillWidth: true
                height: 40
                radius: Tokens.radiusMS
                color: Colors.surface1

                TextEdit {
                    id: apiKeyEdit
                    anchors.fill: parent
                    anchors.margins: 10
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.body
                    verticalAlignment: TextEdit.AlignVCenter
                    selectByMouse: true
                    text: root._apiKey

                    Text {
                        anchors.fill: parent
                        text: "输入 API Key"
                        color: Colors.overlay0
                        font: parent.font
                        visible: !parent.text && !parent.activeFocus
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }

            // 用户名
            Text {
                text: "用户名"
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
            }

            Rectangle {
                Layout.fillWidth: true
                height: 40
                radius: Tokens.radiusMS
                color: Colors.surface1

                TextEdit {
                    id: usernameEdit
                    anchors.fill: parent
                    anchors.margins: 10
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.body
                    verticalAlignment: TextEdit.AlignVCenter
                    selectByMouse: true
                    text: root._username

                    Text {
                        anchors.fill: parent
                        text: "qml-user"
                        color: Colors.overlay0
                        font: parent.font
                        visible: !parent.text && !parent.activeFocus
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }

            Item { Layout.fillHeight: true }

            // 保存按钮
            Rectangle {
                Layout.fillWidth: true
                height: 40
                radius: Tokens.radiusMS
                color: saveSettingsArea.containsMouse
                    ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.25)
                    : Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.15)

                Text {
                    anchors.centerIn: parent
                    text: "保存并连接"
                    color: Colors.blue
                    font.family: Fonts.family
                    font.pixelSize: Fonts.body
                    font.bold: true

                    Behavior on color {
                        ColorAnimation { duration: Tokens.animFast }
                    }
                }

                MouseArea {
                    id: saveSettingsArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root._server = serverEdit.text.trim();
                        root._apiKey = apiKeyEdit.text.trim();
                        root._username = usernameEdit.text.trim() || "qml-user";
                        root.saveState();
                        root.showSettings = false;
                        if (root._server)
                            api.fetchConfigs();
                    }
                }

                Behavior on color {
                    ColorAnimation { duration: Tokens.animFast }
                }
            }
        }
    }
}
