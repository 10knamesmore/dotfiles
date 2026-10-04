pragma Singleton

import "../state"
import QtQuick
import Quickshell
import Quickshell.Io

// 所有屏幕共用一次捕获和录制会话。持有 Python 后端、临时截图和 Portal 请求；
// 浮层读取 snapshot/mode，状态栏读取 recordingState。成功导出或取消后回收原图。
Singleton {
    id: root

    property bool ready: false
    property var dependencies: ({})
    property string mode: ""
    property string target: "region"
    property var snapshot: null
    property string snapshotId: ""
    property string activeScreen: ""
    property var picker: null
    property bool exporting: false
    property string errorText: ""
    property string recordingState: "idle"
    property real recordingStartedAt: 0
    property real clock: Date.now() / 1000
    property string notice: ""
    property bool noticeIsError: false
    property int requestSerial: 0
    property bool waitingForOldBackend: false
    readonly property bool visible: mode !== "" && snapshot !== null
    readonly property bool recordingActive: ["selecting", "starting", "recording", "saving"].indexOf(recordingState) >= 0
    readonly property string recordingLabel: {
        if (recordingState === "recording") {
            const seconds = Math.max(0, Math.floor(clock - recordingStartedAt));
            return Math.floor(seconds / 60).toString().padStart(2, "0") + ":" + (seconds % 60).toString().padStart(2, "0");
        }
        return recordingState === "saving" ? "正在保存" : recordingState === "starting" ? "正在启动" : "选择录制范围";
    }

    signal resetEditor

    function send(command) {
        backend.write(JSON.stringify(command) + "\n");
    }

    function showNotice(text, isError) {
        notice = text;
        noticeIsError = !!isError;
        noticeTimer.restart();
    }

    function releaseSnapshot() {
        const previous = snapshotId;
        snapshot = null;
        snapshotId = "";
        if (previous && ready)
            send({
                type: "release",
                requestId: previous
            });
    }

    function capture() {
        snapshotId = "capture-" + (++requestSerial);
        send({
            type: "snapshot",
            requestId: snapshotId
        });
    }

    // mode 切换复用同一张冻结原图，不截到自己的工具条。
    function open(nextMode, nextTarget) {
        if (!ready) {
            showNotice("捕获服务尚未就绪", true);
            return;
        }
        if (picker || exporting || mode === "record-wait")
            return;
        if (nextMode === "record-setup" && recordingActive) {
            showNotice("已有录制正在进行，可从状态栏停止", false);
            return;
        }
        errorText = "";
        notice = "";
        mode = nextMode;
        target = nextTarget || "region";
        resetEditor();
        if (!snapshot && !snapshotId)
            capture();
        console.info("[capture] open mode=" + mode);
    }

    function setTarget(value) {
        target = value;
        resetEditor();
    }

    function cancel() {
        if (exporting)
            return;
        if (picker)
            send({
                type: "picker_cancel",
                requestId: picker.requestId
            });
        if (mode === "record-wait")
            send({
                type: "stop_recording"
            });
        mode = "";
        errorText = "";
        releaseSnapshot();
    }

    function exportImage(path, crop, action) {
        exporting = true;
        errorText = "";
        send({
            type: "export",
            requestId: snapshotId,
            imagePath: path,
            crop: crop,
            action: action
        });
    }

    function copyColor(value) {
        exporting = true;
        send({
            type: "copy_color",
            value: value
        });
    }

    // 先确定音频，再让 GSR 请求一次 Portal 选区；不预先批准任何共享请求。
    function prepareRecording(systemAudio, microphone, cursor) {
        if (!dependencies.gpuScreenRecorder) {
            errorText = "尚未安装 gpu-screen-recorder";
            return;
        }
        errorText = "";
        mode = "record-wait";
        send({
            type: "record_prepare",
            systemAudio: systemAudio,
            microphone: microphone,
            cursor: cursor
        });
    }

    function confirmPicker(selection) {
        if (!picker || exporting)
            return;
        exporting = true;
        send({
            type: "picker_select",
            requestId: picker.requestId,
            selection: selection
        });
    }

    function stopRecording() {
        if (recordingActive && recordingState !== "saving")
            send({
                type: "stop_recording"
            });
    }

    function errorMessage(event) {
        if (event.code === "missing_dependency")
            return "缺少依赖：" + (event.dependency || "捕获工具");
        switch (event.code) {
        case "portal_monitor_unavailable":
        case "portal_monitor_failed":
            return "无法连接屏幕共享服务，请检查 Hyprland Portal";
        case "picker_not_started":
            return "共享选择器未启动，请检查 Portal 配置";
        case "picker_not_identified":
            return "未能确认录屏的共享请求，录制已停止";
        case "first_frame_timeout":
        case "first_frame_missing":
            return "未收到录制画面，请检查 Portal 和显卡驱动";
        case "recorder_exited":
            return "录制异常结束，请查看捕获服务日志";
        case "recorder_stop_timeout":
            return "录制未能完成保存";
        case "invalid_selection":
            return "所选窗口或显示器已变化，请重新选择";
        case "no_outputs":
            return "没有可捕获的显示器";
        }
        return event.operation === "snapshot" ? "获取屏幕画面失败" : event.operation === "export" ? "图片导出失败" : event.operation === "copy_color" ? "颜色复制失败" : "捕获操作失败，请查看日志";
    }

    function receive(event) {
        switch (event.type) {
        case "ready":
            ready = true;
            waitingForOldBackend = false;
            dependencies = event.dependencies;
            break;
        case "snapshot":
            if (event.requestId !== snapshotId) {
                send({
                    type: "release",
                    requestId: event.requestId
                });
                return;
            }
            snapshot = event;
            const pointer = event.pointer;
            const screen = event.screens.find(s => pointer.x >= s.x && pointer.y >= s.y && pointer.x < s.x + s.width && pointer.y < s.y + s.height);
            activeScreen = (screen || event.screens[0]).name;
            PanelState.closeAll();
            break;
        case "exported":
            if (event.requestId !== snapshotId)
                return;
            exporting = false;
            mode = "";
            releaseSnapshot();
            showNotice(event.action === "copy" ? "截图已复制" : "截图已保存：" + event.path, false);
            break;
        case "copied_color":
            exporting = false;
            mode = "";
            releaseSnapshot();
            showNotice("颜色已复制：" + event.value, false);
            break;
        case "picker_request":
            if (mode && mode !== "record-wait") {
                send({
                    type: "picker_cancel",
                    requestId: event.requestId
                });
                showNotice("请先完成当前捕获，再发起屏幕共享", true);
                return;
            }
            picker = event;
            mode = "portal";
            target = "window";
            exporting = false;
            errorText = "";
            resetEditor();
            if (!snapshot)
                capture();
            break;
        case "picker_closed":
            if (picker && picker.requestId === event.requestId) {
                picker = null;
                exporting = false;
                mode = "";
                releaseSnapshot();
            }
            break;
        case "recording":
            recordingState = event.state;
            if (event.startedAt !== undefined) {
                recordingStartedAt = event.startedAt;
                clock = Date.now() / 1000;
            }
            if (event.state === "idle" && event.path)
                showNotice("录屏已保存：" + event.path, false);
            if ((event.state === "error" || event.state === "idle") && mode === "record-wait") {
                mode = "record-setup";
            }
            break;
        case "error":
            if (event.code === "server_already_running") {
                waitingForOldBackend = true;
                return;
            }
            console.warn("[capture] operation=" + event.operation + " code=" + event.code);
            if (event.requestId && event.requestId !== snapshotId && (!picker || event.requestId !== picker.requestId))
                return;
            exporting = false;
            const message = errorMessage(event);
            if (event.operation === "snapshot") {
                mode = "";
                releaseSnapshot();
                if (picker)
                    send({
                        type: "picker_cancel",
                        requestId: picker.requestId
                    });
                showNotice(message, true);
            } else if (visible) {
                errorText = message;
            } else {
                showNotice(message, true);
            }
            break;
        }
    }

    Process {
        id: backend
        command: ["python3", "-u", Quickshell.shellPath("capture/backend/launcher.py")]
        running: true
        stdinEnabled: true
        stdout: SplitParser {
            onRead: line => root.receive(JSON.parse(line))
        }
        stderr: SplitParser {
            onRead: line => console.info("[capture-backend] " + line)
        }
        onExited: exitCode => {
            root.ready = false;
            root.exporting = false;
            root.mode = "";
            root.snapshot = null;
            root.snapshotId = "";
            root.picker = null;
            if (root.waitingForOldBackend) {
                restartAfterReload.restart();
            } else {
                root.recordingState = "error";
                root.showNotice("捕获服务已停止，请重新加载 Quickshell", true);
                console.warn("[capture] backend exited code=" + exitCode);
            }
        }
    }

    // Quickshell 热重载时旧后端可能仍在保存视频；仅针对其持锁响应等待交接。
    Timer {
        id: restartAfterReload
        interval: 500
        onTriggered: backend.running = true
    }
    Timer {
        interval: 1000
        running: root.recordingState === "recording"
        repeat: true
        onTriggered: root.clock = Date.now() / 1000
    }
    Timer {
        id: noticeTimer
        interval: 6500
        onTriggered: root.notice = ""
    }
    IpcHandler {
        target: "capture"
        function screenshot(): void {
            root.open("screenshot", "region");
        }
        function window(): void {
            root.open("screenshot", "window");
        }
        function color(): void {
            root.open("color", "region");
        }
        function record(): void {
            root.open("record-setup", "region");
        }
        function stop(): void {
            root.stopRecording();
        }
        function cancel(): void {
            root.cancel();
        }
    }
}
