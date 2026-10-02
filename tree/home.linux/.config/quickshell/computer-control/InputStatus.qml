import QtQuick
import QtQuick.Shapes

// 只读 agent 持有状态与短暂操作摘要；文本事件只呈现字符数。
Rectangle {
    id: root

    property ControlConnection connection: null
    readonly property var heldKeys: connection ? connection.input.keys : []
    readonly property var heldButtons: connection ? connection.input.buttons : []
    readonly property var buttonNames: ({ left: "左键", right: "右键", middle: "中键", back: "后退键", forward: "前进键" })
    readonly property string keysDescription: heldKeys.length ? "按住的键：" + heldKeys.join(" + ") : "没有按住的键"
    readonly property string buttonsDescription: heldButtons.length
        ? "按住的按钮：" + heldButtons.map(button => buttonNames[button]).join("、") : "鼠标按钮已松开"
    readonly property string recentDescription: {
        if (!connection || !connection.input.transientKind)
            return "仅显示 agent 的虚拟输入";
        if (connection.input.transientKind === "text")
            return "输入文本 · " + connection.input.characters + " 字符";
        if (connection.input.transientKind === "keys")
            return "最近按键 · " + connection.input.recentKeys.join(" + ");
        const amount = connection.input.scrollAmount;
        return (connection.input.horizontalScroll ? "水平滚动" : "垂直滚动") + " · " + (amount > 0 ? "+" : "") + amount;
    }

    implicitWidth: 470
    implicitHeight: 88
    height: implicitHeight
    radius: 13
    color: "#f5181825"
    border.color: "#9045475a"
    border.width: 1
    Accessible.role: Accessible.Grouping
    Accessible.name: "Agent 输入状态"

    Item {
        x: 18
        anchors.verticalCenter: parent.verticalCenter
        width: 25
        height: 38
        Accessible.ignored: true
        Shape {
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer
            ShapePath {
                strokeColor: "transparent"
                fillColor: root.heldButtons.indexOf("left") >= 0 ? "#b4befe" : "#313244"
                startX: 3; startY: 16
                PathLine { x: 3; y: 11 }
                PathQuad { x: 12; y: 2; controlX: 3; controlY: 2 }
                PathLine { x: 12; y: 16 }
                PathLine { x: 3; y: 16 }
            }
            ShapePath {
                strokeColor: "transparent"
                fillColor: root.heldButtons.indexOf("right") >= 0 ? "#b4befe" : "#313244"
                startX: 14; startY: 2
                PathQuad { x: 23; y: 11; controlX: 23; controlY: 2 }
                PathLine { x: 23; y: 16 }
                PathLine { x: 14; y: 16 }
                PathLine { x: 14; y: 2 }
            }
            ShapePath {
                strokeColor: "#7f849c"
                strokeWidth: 1.2
                fillColor: "transparent"
                startX: 3; startY: 12
                PathCubic { x: 23; y: 12; control1X: 3; control1Y: -2; control2X: 23; control2Y: -2 }
                PathLine { x: 23; y: 26 }
                PathCubic { x: 3; y: 26; control1X: 23; control1Y: 40; control2X: 3; control2Y: 40 }
                PathLine { x: 3; y: 12 }
            }
        }
        Rectangle {
            x: 11; y: 8
            width: 4; height: 7; radius: 2
            color: root.connection && root.connection.input.transientKind === "scroll" ? "#89dceb"
                : root.heldButtons.indexOf("middle") >= 0 ? "#b4befe" : "#585b70"
        }
    }

    Rectangle {
        x: 59
        anchors.verticalCenter: parent.verticalCenter
        width: 1
        height: 48
        color: "#6045475a"
        Accessible.ignored: true
    }

    Column {
        x: 75
        anchors.verticalCenter: parent.verticalCenter
        width: parent.width - x - 18
        spacing: 5
        Text {
            width: parent.width
            text: root.keysDescription
            textFormat: Text.PlainText
            elide: Text.ElideRight
            color: root.heldKeys.length ? "#dce2ff" : "#a6adc8"
            font.pixelSize: 12
            font.weight: root.heldKeys.length ? Font.DemiBold : Font.Normal
            Accessible.role: Accessible.StaticText
            Accessible.name: text
        }
        Text {
            width: parent.width
            text: root.buttonsDescription
            textFormat: Text.PlainText
            elide: Text.ElideRight
            color: root.heldButtons.length ? "#b4befe" : "#9399b2"
            font.pixelSize: 11
            Accessible.role: Accessible.StaticText
            Accessible.name: text
        }
        Text {
            width: parent.width
            text: root.recentDescription
            textFormat: Text.PlainText
            elide: Text.ElideRight
            color: root.connection && root.connection.input.transientKind ? "#b4befe" : "#7f849c"
            font.pixelSize: 10
            Accessible.role: Accessible.StaticText
            Accessible.name: text
        }
    }
}
