pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    property bool ready: false
    property var adapters: []
    property var devices: []
    property string selectedAdapter: ""
    property string scanningAdapter: ""
    readonly property var adapter: adapters.find(item => item.path === selectedAdapter) ?? adapters[0] ?? null
    readonly property bool powered: adapter?.powered ?? false
    readonly property bool scanning: scanningAdapter === adapter?.path
    readonly property var connectedDevices: devices.filter(device => device.connected)
    property var operations: ({})
    property var errors: ({})
    property var prompt: null
    property string serviceError: ""
    property int nextRequest: 0
    readonly property var progressOperations: Object.values(operations).filter(operation => operation.action !== "trust")
    readonly property bool busy: progressOperations.length > 0

    function request(action, device = "", value = false) {
        const id = String(++nextRequest);
        const command = { id, action, device: device || null, adapter: adapter?.path ?? null, value };
        operations = Object.assign({}, operations, { [id]: { device, action, value, adapter: command.adapter, stage: action } });
        errors = Object.assign({}, errors, { [device]: "" });
        backend.write(JSON.stringify(command) + "\n");
    }

    function respond(accept, value = "") {
        backend.write(JSON.stringify({ action: "respond", requestId: prompt.requestId, accept, value }) + "\n");
    }

    function progressForDevice(path) {
        const matching = progressOperations.filter(operation => operation.device === path);
        return matching.find(operation => operation.action === "cancel") ?? matching[0] ?? null;
    }

    function deviceFor(path) {
        return devices.find(device => device.path === path) ?? null;
    }

    function receive(event) {
        if (event.type === "ready") {
            ready = true;
        } else if (event.type === "state") {
            adapters = event.adapters;
            devices = event.devices;
            if (!adapters.some(item => item.path === scanningAdapter && item.powered && item.discovering))
                scanningAdapter = "";
        } else if (event.type === "prompt") {
            prompt = event;
        } else if (event.type === "promptClosed") {
            if (prompt?.requestId === event.requestId)
                prompt = null;
        } else if (event.type === "operation") {
            const updated = Object.assign({}, operations);
            if (event.stage === "done" || event.stage === "error") {
                const operation = updated[event.id];
                if (event.stage === "done" && operation?.action === "scan")
                    scanningAdapter = operation.value ? operation.adapter : "";
                delete updated[event.id];
                if (event.stage === "error")
                    errors = Object.assign({}, errors, { [event.device]: event.error });
            } else {
                updated[event.id] = event;
            }
            operations = updated;
        }
    }

    Process {
        id: backend
        command: ["desktop-backend", "bluetooth"]
        running: true
        stdinEnabled: true
        stdout: SplitParser {
            onRead: line => root.receive(JSON.parse(line))
        }
        stderr: SplitParser {
            onRead: line => {
                root.serviceError += (root.serviceError ? "\n" : "") + line;
                console.error(line);
            }
        }
        onExited: (exitCode, exitStatus) => {
            root.ready = false;
            root.operations = {};
            root.scanningAdapter = "";
            root.prompt = null;
            root.serviceError = root.serviceError || "蓝牙服务已停止（" + exitCode + "）";
        }
    }
}
