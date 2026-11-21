import QtQuick
import Quickshell.Io

Socket {
    id: root

    required property var service
    property int clientPid: 0
    property string phase: "hello"

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
        closeConnection(event.type === "close" ? "client-close" : "unexpected-message");
    }

    onConnectedChanged: {
        if (connected) {
            service.connected(root);
        } else {
            phase = "closed";
            disconnectGrace.stop();
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
}
