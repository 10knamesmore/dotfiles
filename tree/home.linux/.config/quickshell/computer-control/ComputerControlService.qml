import QtQuick
import Quickshell
import Quickshell.Io

// 同一 Hyprland 只接受一个控制者；撤销后等旧连接离开再接收下一位。
// SocketServer 重载或销毁会关闭旧连接，客户端必须把 EOF 视为撤销。
Scope {
    id: root

    readonly property string runtimeDirectory: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property string instanceSignature: Quickshell.env("HYPRLAND_INSTANCE_SIGNATURE")
    readonly property string socketPath: runtimeDirectory + "/hypr/" + instanceSignature + "/computer-use.sock"
    property ControlConnection controller: null
    readonly property bool controlling: controller !== null && (controller.phase === "overlay" || controller.phase === "ready")
    readonly property ControlConnection currentConnection: controlling ? controller : null

    function connected(connection) {
        console.info("[computer-control] connected");
    }

    function disconnected(connection) {
        if (controller === connection)
            controller = null;
        console.info("[computer-control] disconnected pid=" + connection.clientPid);
    }

    function register(connection) {
        if (controller !== null) {
            connection.rejectBusy();
            return;
        }
        controller = connection;
        console.info("[computer-control] hello pid=" + connection.clientPid);
        confirmReady();
    }

    // 取得控制权前，所有显示器必须已经提交提示层的一帧。
    function confirmReady() {
        if (!controlling || !Quickshell.screens.length || overlays.instances.length !== Quickshell.screens.length)
            return;
        for (const overlay of overlays.instances) {
            if (!overlay.presented)
                return;
        }
        controller.confirmReady();
    }

    function revoke(reason) {
        if (!controlling)
            return;
        console.info("[computer-control] revoke reason=" + reason);
        controller.revoke();
    }

    SocketServer {
        id: server
        path: root.socketPath
        active: root.runtimeDirectory !== "" && root.instanceSignature !== ""
        handler: ControlConnection {
            service: root
        }
        onActiveChanged: console.info("[computer-control] listening=" + active)
    }

    Variants {
        id: overlays
        model: Quickshell.screens
        delegate: ControlOverlay {
            required property ShellScreen modelData
            screen: modelData
            service: root
        }
    }

    Connections {
        target: Quickshell
        function onScreensChanged() {
            if (!Quickshell.screens.length)
                root.revoke("no-screens");
        }
    }
}
