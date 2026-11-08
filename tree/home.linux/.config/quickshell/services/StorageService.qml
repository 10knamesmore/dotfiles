pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    property bool ready: false
    property var drives: []
    property var settings: ({
            autoMount: false,
            notifications: true,
            volumes: {}
        })
    property var operations: ({})
    property var errors: ({})
    property string serviceError: ""
    property int nextRequest: 0
    readonly property int mountedCount: drives.reduce((count, drive) => count + drive.volumes.filter(volume => volume.mountPoints.length > 0).length, 0)
    readonly property bool configuring: Object.values(operations).some(operation => operation.action === "settings" || operation.action === "preference")

    signal noticeReceived(var notice)

    function request(action, target = "", options = {}) {
        const id = String(++nextRequest);
        const command = Object.assign({
            id,
            action,
            target
        }, options);
        operations = Object.assign({}, operations, {
            [id]: command
        });
        errors = Object.assign({}, errors, {
            [target]: ""
        });
        backend.write(JSON.stringify(command) + "\n");
    }

    function configure(autoMount, notifications) {
        request("settings", "", {
            autoMount,
            notifications
        });
    }

    function preference(key) {
        return settings.volumes[key] ?? {
            autoMount: "inherit",
            readOnly: false
        };
    }

    function savePreference(key, autoMount, readOnly) {
        request("preference", key, {
            autoMount,
            readOnly
        });
    }

    function busyForDrive(drive) {
        return Object.values(operations).some(operation => operation.target === drive.path || drive.volumes.some(volume => volume.path === operation.target));
    }

    function mountCandidate(target) {
        const volumes = drives.reduce((volumes, drive) => volumes.concat(drive.path === target ? drive.volumes : drive.volumes.filter(volume => volume.path === target)), []);
        const candidates = volumes.filter(volume => volume.mountable && volume.mountPoints.length === 0);
        return candidates.length === 1 ? candidates[0] : null;
    }

    function busyForTarget(target) {
        const drive = drives.find(drive => drive.path === target || drive.volumes.some(volume => volume.path === target));
        return drive ? busyForDrive(drive) : false;
    }

    function openVolume(volume) {
        Qt.openUrlExternally("file://" + volume.mountPoints[0].split("/").map(encodeURIComponent).join("/"));
    }

    function formatSize(bytes) {
        if (bytes < 1024 * 1024 * 1024)
            return (bytes / (1024 * 1024)).toFixed(0) + " MiB";
        return (bytes / (1024 * 1024 * 1024)).toFixed(1) + " GiB";
    }

    function receive(event) {
        if (event.type === "state") {
            drives = event.drives;
            settings = event.settings;
        } else if (event.type === "ready") {
            ready = true;
            console.info("[storage] ready; drives=" + drives.length);
        } else if (event.type === "notice") {
            noticeReceived(event);
        } else if (event.type === "operation") {
            console.info("[storage]", event.action, event.stage, event.target);
            const updated = Object.assign({}, operations);
            if (event.stage === "done" || event.stage === "error") {
                delete updated[event.id];
                errors = Object.assign({}, errors, {
                    [event.target]: event.stage === "error" ? event.error : ""
                });
            } else {
                updated[event.id] = event;
                errors = Object.assign({}, errors, {
                    [event.target]: ""
                });
            }
            operations = updated;
        }
    }

    Process {
        id: backend
        command: ["desktop-backend", "storage"]
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
            root.serviceError = root.serviceError || "存储服务已停止（" + exitCode + "）";
        }
    }
}
