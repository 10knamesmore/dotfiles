import QtQuick
import Quickshell.Io

// 单个 NDJSON 客户端的握手和输入状态；只接收 agent 发来的展示事件。
// 不保留原始帧或文本，UI 仅能读取持有键、按钮及文本字符数。
Socket {
    id: root

    required property var service
    property int clientPid: 0
    property string phase: "hello"
    property var pointer: null // 输出内逻辑坐标 { monitor, x, y }，收到 pointer 前为空。
    property QtObject input: QtObject {
        property var keys: []
        property var recentKeys: [] // 保留最近按下的组合键，避免相邻 down/up 在一帧内消失。
        property var buttons: []
        property bool clickFeedback: false
        property string transientKind: ""
        property int characters: 0
        property real scrollAmount: 0
        property bool horizontalScroll: false
    }

    function confirmReady() {
        if (phase !== "overlay" || !connected)
            return;
        phase = "ready";
        write('{"type":"ready"}\n');
        flush();
        console.info("[computer-control] ready pid=" + clientPid);
    }

    function closeConnection(reason) {
        phase = "closed";
        console.info("[computer-control] close pid=" + clientPid + " reason=" + reason);
        connected = false;
    }

    function rejectBusy() {
        phase = "closed";
        write('{"type":"busy"}\n');
        flush();
        disconnectGrace.restart();
        console.info("[computer-control] busy pid=" + clientPid);
    }

    function revoke() {
        if (!connected || phase === "closed")
            return;
        phase = "closed";
        write('{"type":"revoke"}\n');
        flush();
        // flush 不等待客户端读取；名额保留到 disconnected，SDK 另持 flock 直至输入清理完毕。
        disconnectGrace.restart();
    }

    function validNumber(value) {
        return typeof value === "number" && Number.isFinite(value);
    }

    function receive(line) {
        if (phase === "closed")
            return;
        let event;
        try {
            event = JSON.parse(line);
        } catch (_) {
            closeConnection("invalid-json");
            return;
        }
        if (!event || typeof event !== "object" || Array.isArray(event)) {
            closeConnection("invalid-frame");
            return;
        }
        if (phase === "hello") {
            if (event.type !== "hello" || !Number.isInteger(event.pid) || event.pid <= 0) {
                closeConnection("expected-hello");
                return;
            }
            clientPid = event.pid;
            phase = "overlay";
            service.register(root);
            return;
        }
        if (event.type === "close") {
            closeConnection("client-close");
            return;
        }
        if (phase !== "ready") {
            closeConnection("input-before-ready");
            return;
        }
        switch (event.type) {
        case "pointer":
            if (typeof event.monitor !== "string" || !event.monitor.length || !validNumber(event.x) || !validNumber(event.y)) {
                closeConnection("invalid-pointer");
                return;
            }
            pointer = { monitor: event.monitor, x: event.x, y: event.y };
            break;
        case "button":
            if (["left", "right", "middle", "back", "forward"].indexOf(event.button) < 0 || typeof event.pressed !== "boolean") {
                closeConnection("invalid-button");
                return;
            }
            input.buttons = input.buttons.filter(button => button !== event.button);
            if (event.pressed) {
                input.buttons = input.buttons.concat([event.button]);
                input.clickFeedback = true;
                clickFeedbackTimer.restart();
            }
            break;
        case "keys":
            if (!Array.isArray(event.keys) || !event.keys.every(key => typeof key === "string")) {
                closeConnection("invalid-keys");
                return;
            }
            const newlyPressed = event.keys.some(key => input.keys.indexOf(key) < 0);
            input.keys = event.keys;
            if (newlyPressed) {
                input.recentKeys = event.keys;
                input.transientKind = "keys";
                transientTimer.restart();
            }
            break;
        case "text":
            if (!Number.isInteger(event.characters) || event.characters < 0) {
                closeConnection("invalid-text-count");
                return;
            }
            input.characters = event.characters;
            input.transientKind = "text";
            transientTimer.restart();
            break;
        case "scroll":
            if (!validNumber(event.amount) || typeof event.horizontal !== "boolean") {
                closeConnection("invalid-scroll");
                return;
            }
            input.scrollAmount = event.amount;
            input.horizontalScroll = event.horizontal;
            input.transientKind = "scroll";
            transientTimer.restart();
            break;
        default:
            closeConnection("unknown-event");
            return;
        }
        console.info("[computer-control] input pid=" + clientPid + " type=" + event.type);
    }

    onConnectedChanged: {
        if (connected) {
            service.connected(root);
        } else {
            phase = "closed";
            disconnectGrace.stop();
            clickFeedbackTimer.stop();
            transientTimer.stop();
            service.disconnected(root);
        }
    }
    onError: error => {
        // QLocalSocket::PeerClosedError (1) 随正常 EOF 到达，由 disconnected 记录。
        if (error !== 1) {
            console.warn("[computer-control] socket error pid=" + clientPid + " code=" + error);
            closeConnection("socket-error");
        }
    }

    parser: SplitParser {
        splitMarker: "\n"
        onRead: message => root.receive(message)
    }

    property Timer disconnectGrace: Timer {
        interval: 250
        onTriggered: root.connected = false
    }
    property Timer clickFeedbackTimer: Timer {
        interval: 150
        onTriggered: root.input.clickFeedback = false
    }
    property Timer transientTimer: Timer {
        interval: 1600
        onTriggered: root.input.transientKind = ""
    }
}
