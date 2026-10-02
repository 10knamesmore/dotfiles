import QtQuick
import QtQuick.Shapes

// 仅呈现当前连接的虚拟坐标。新目标从当前画面位置重新起弧，不排队或阻塞实际输入。
Item {
    id: root

    property ControlConnection connection: null
    required property string screenName
    readonly property bool onThisScreen: connection !== null && connection.pointer !== null && connection.pointer.monitor === screenName
    property bool positioned: false
    property real progress: 1
    property var curve: ({
            startX: 0,
            startY: 0,
            control1X: 0,
            control1Y: 0,
            control2X: 0,
            control2Y: 0,
            endX: 0,
            endY: 0
        })
    readonly property real cursorX: bezier(curve.startX, curve.control1X, curve.control2X, curve.endX)
    readonly property real cursorY: bezier(curve.startY, curve.control1Y, curve.control2Y, curve.endY)
    readonly property bool pressed: connection !== null && (connection.input.clickFeedback || connection.input.buttons.length > 0)

    visible: onThisScreen && positioned
    Accessible.ignored: true

    function bezier(start, control1, control2, end) {
        const t = progress;
        const u = 1 - t;
        return u * u * u * start + 3 * u * u * t * control1 + 3 * u * t * t * control2 + t * t * t * end;
    }

    function movePointer() {
        const target = connection ? connection.pointer : null;
        if (!target || target.monitor !== screenName) {
            motion.stop();
            positioned = false;
            return;
        }
        const fromX = positioned ? cursorX : target.x;
        const fromY = positioned ? cursorY : target.y;
        const dx = target.x - fromX;
        const dy = target.y - fromY;
        const distance = Math.hypot(dx, dy);
        const bend = Math.min(distance * 0.2, 80) * (dx >= 0 ? -1 : 1);
        const normalX = distance ? -dy / distance : 0;
        const normalY = distance ? dx / distance : 0;
        motion.stop();
        curve = {
            startX: fromX,
            startY: fromY,
            control1X: fromX + dx * 0.26 + normalX * bend,
            control1Y: fromY + dy * 0.26 + normalY * bend,
            control2X: fromX + dx * 0.72 + normalX * bend * 0.65,
            control2Y: fromY + dy * 0.72 + normalY * bend * 0.65,
            endX: target.x,
            endY: target.y
        };
        positioned = true;
        progress = 0;
        motion.duration = Math.max(180, Math.min(540, distance * 0.5));
        motion.start();
    }

    onConnectionChanged: {
        positioned = false;
        movePointer();
    }
    onScreenNameChanged: movePointer()
    Connections {
        target: root.connection
        function onPointerChanged() {
            root.movePointer();
        }
    }

    NumberAnimation {
        id: motion
        target: root
        property: "progress"
        to: 1
        easing.type: Easing.OutCubic
    }

    Item {
        x: root.cursorX
        y: root.cursorY
        width: 54
        height: 47
        Accessible.role: Accessible.Graphic
        Accessible.name: "Pi 虚拟光标"
        Accessible.description: root.pressed ? "鼠标按钮按下" : "鼠标按钮松开"

        Shape {
            width: 28
            height: 36
            scale: root.pressed ? 0.86 : 1
            transformOrigin: Item.TopLeft
            preferredRendererType: Shape.CurveRenderer
            Accessible.ignored: true
            Behavior on scale {
                NumberAnimation {
                    duration: 70
                    easing.type: Easing.OutCubic
                }
            }
            ShapePath {
                strokeColor: "#181825"
                strokeWidth: 1.6
                fillColor: root.pressed ? "#e4e8ff" : "#b4befe"
                joinStyle: ShapePath.RoundJoin
                startX: 1
                startY: 1
                PathLine {
                    x: 24
                    y: 20
                }
                PathLine {
                    x: 14
                    y: 21
                }
                PathLine {
                    x: 10
                    y: 32
                }
                PathLine {
                    x: 1
                    y: 1
                }
            }
        }
        Rectangle {
            x: 21
            y: 28
            width: 28
            height: 19
            radius: 6
            topLeftRadius: 2
            color: "#b4befe"
            Accessible.ignored: true
            Text {
                anchors.centerIn: parent
                text: "Pi"
                color: "#181825"
                font.pixelSize: 10
                font.bold: true
                Accessible.ignored: true
            }
        }
    }
}
