import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

// 桌面浮动番茄钟 — 右下角，时钟上方
PanelWindow {
    id: root

    required property var modelData
    screen: modelData

    // ── 状态 ──
    property string mode: "work" // "work" | "break" | "longBreak"
    property bool running: false
    property int totalSeconds: 25 * 60
    property int remainingSeconds: 25 * 60
    property int completedPomodoros: 0

    readonly property int ringSize: 180
    readonly property color modeColor: mode === "work" ? Colors.red : mode === "break" ? Colors.green : Colors.blue
    readonly property string modeLabel: mode === "work" ? "专注" : mode === "break" ? "休息" : "长休息"

    function modeDurations(m) {
        if (m === "work") return 25 * 60;
        if (m === "break") return 5 * 60;
        return 15 * 60;
    }

    function resetTimer() {
        running = false;
        totalSeconds = modeDurations(mode);
        remainingSeconds = totalSeconds;
        ring.requestPaint();
    }

    function switchMode(m) {
        mode = m;
        resetTimer();
    }

    function skipToNext() {
        running = false;
        if (mode === "work") {
            completedPomodoros++;
            switchMode(completedPomodoros % 4 === 0 ? "longBreak" : "break");
        } else {
            switchMode("work");
        }
    }

    function formatTime(secs) {
        let m = Math.floor(secs / 60);
        let s = secs % 60;
        return (m < 10 ? "0" : "") + m + ":" + (s < 10 ? "0" : "") + s;
    }

    aboveWindows: false
    anchors.top: true
    anchors.left: true
    implicitWidth: 260
    implicitHeight: 310
    margins.top: 610
    margins.left: 180
    visible: PanelState.pomodoroVisible
    focusable: false
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"

    // ── 计时器 ──
    Timer {
        interval: 1000
        running: root.running
        repeat: true
        onTriggered: {
            if (root.remainingSeconds > 0) {
                root.remainingSeconds--;
                ring.requestPaint();
            }
            if (root.remainingSeconds <= 0) {
                root.running = false;
                notifyProc.running = true;
                root.skipToNext();
            }
        }
    }

    Process {
        id: notifyProc

        command: ["notify-send", "-u", "critical", "-a", "QuickShell", "番茄钟",
            root.mode === "work" ? "专注时间结束，休息一下吧！" : "休息结束，开始专注！"]
    }

    // ── 主体容器 ──
    Rectangle {
        id: body

        anchors.centerIn: parent
        width: 240
        height: 290
        radius: Tokens.radiusXL
        color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, Tokens.panelAlpha)
        border.color: Qt.rgba(1, 1, 1, Tokens.borderAlpha)
        border.width: 1

        SoftShadow {
            anchors.fill: parent
            radius: parent.radius
        }

        InnerGlow {}

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 10
            spacing: 6

            // ── 模式标签 ──
            Text {
                Layout.alignment: Qt.AlignHCenter
                text: root.modeLabel
                color: root.modeColor
                font.family: Fonts.family
                font.pixelSize: Fonts.small
                font.weight: Font.DemiBold

                Behavior on color {
                    ColorAnimation { duration: Tokens.animNormal }
                }
            }

            // ── 进度环 ──
            Item {
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: root.ringSize
                Layout.preferredHeight: root.ringSize

                Canvas {
                    id: ring

                    anchors.fill: parent

                    onPaint: {
                        let ctx = getContext("2d");
                        let w = width;
                        let h = height;
                        let cx = w / 2;
                        let cy = h / 2;
                        let r = Math.min(cx, cy) - 8;
                        let lineW = 5;

                        ctx.clearRect(0, 0, w, h);

                        // 背景环
                        ctx.beginPath();
                        ctx.arc(cx, cy, r, 0, 2 * Math.PI);
                        ctx.strokeStyle = Colors.surface1.toString();
                        ctx.lineWidth = lineW;
                        ctx.stroke();

                        // 进度环
                        let progress = root.totalSeconds > 0 ? (1 - root.remainingSeconds / root.totalSeconds) : 0;
                        if (progress > 0) {
                            let startAngle = -Math.PI / 2;
                            let endAngle = startAngle + progress * 2 * Math.PI;
                            ctx.beginPath();
                            ctx.arc(cx, cy, r, startAngle, endAngle);
                            ctx.strokeStyle = root.modeColor.toString();
                            ctx.lineWidth = lineW;
                            ctx.lineCap = "round";
                            ctx.stroke();
                        }
                    }
                }

                // 中间时间
                Text {
                    anchors.centerIn: parent
                    text: root.formatTime(root.remainingSeconds)
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.h1
                    font.weight: Font.Bold
                }
            }

            // ── 番茄计数 ──
            Row {
                Layout.alignment: Qt.AlignHCenter
                spacing: 4

                Repeater {
                    model: 4

                    delegate: Rectangle {
                        required property int index

                        width: 8
                        height: 8
                        radius: 4
                        color: index < (root.completedPomodoros % 4) ? Colors.red : Colors.surface1

                        Behavior on color {
                            ColorAnimation { duration: Tokens.animFast }
                        }
                    }
                }
            }

            // ── 控制按钮 ──
            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 8

                // 重置
                PomodoroBtn {
                    text: "󰜉"
                    tipColor: Colors.yellow
                    onClicked: root.resetTimer()
                }

                // 开始/暂停
                PomodoroBtn {
                    text: root.running ? "󰏤" : "󰐊"
                    tipColor: root.running ? Colors.peach : Colors.green
                    onClicked: root.running = !root.running
                }

                // 跳过
                PomodoroBtn {
                    text: "󰒭"
                    tipColor: Colors.blue
                    onClicked: root.skipToNext()
                }
            }
        }
    }

    // 可复用控制按钮
    component PomodoroBtn: Rectangle {
        property string text: ""
        property color tipColor: Colors.overlay1

        signal clicked()

        width: 32
        height: 32
        radius: Tokens.radiusFull
        color: btnArea.containsMouse ? Qt.rgba(tipColor.r, tipColor.g, tipColor.b, 0.15) : "transparent"

        Text {
            anchors.centerIn: parent
            text: parent.text
            color: btnArea.containsMouse ? parent.tipColor : Colors.overlay1
            font.family: Fonts.family
            font.pixelSize: Fonts.iconLarge

            Behavior on color {
                ColorAnimation { duration: Tokens.animFast }
            }
        }

        MouseArea {
            id: btnArea

            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }

        Behavior on color {
            ColorAnimation { duration: Tokens.animFast }
        }
    }
}
