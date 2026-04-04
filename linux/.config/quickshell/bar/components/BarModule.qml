import "../../theme"
import QtQuick
import QtQuick.Layouts

// 基础模块包装：圆角背景 + 左侧彩色边框 + hover 动画 + 阴影 + 状态色调
// 用法：BarModule { accentColor: Colors.blue; RowLayout { ... } }
Rectangle {
    id: root

    property color accentColor: Colors.blue
    property color backgroundColor: Colors.surface0
    property color tintColor: "transparent"
    property bool hovered: hoverArea.containsMouse
    default property alias contents: inner.data

    // 外部可连接的点击信号
    signal clicked(var mouse)
    signal rightClicked(var mouse)
    signal scrolled(int delta)

    radius: 16
    color: hovered ? Colors.surface1 : backgroundColor
    implicitHeight: 36

    // 阴影（伪 box-shadow，轻量级）
    Rectangle {
        anchors.fill: parent
        anchors.margins: -1
        anchors.topMargin: 0
        anchors.bottomMargin: -3
        z: -1
        radius: root.radius + 2
        color: "#000000"
        opacity: root.hovered ? 0.22 : 0.15

        Behavior on opacity {
            NumberAnimation {
                duration: 300
                easing.type: Easing.OutCubic
            }

        }

    }

    // 状态色调叠加层（电池/systemd 等用）
    Rectangle {
        anchors.fill: parent
        radius: root.radius
        color: root.tintColor
        visible: root.tintColor !== Qt.rgba(0, 0, 0, 0)

        Behavior on color {
            ColorAnimation {
                duration: 500
                easing.type: Easing.OutCubic
            }

        }

    }

    // 左侧彩色边框
    Rectangle {
        width: 4
        height: parent.height * 0.6
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        radius: 2
        color: root.accentColor
        opacity: root.hovered ? 1 : 0.7

        Behavior on opacity {
            NumberAnimation {
                duration: 300
            }

        }

    }

    // 内容区（左移 4px 避开边框，再加 14px padding）
    Item {
        id: inner

        anchors {
            fill: parent
            leftMargin: 18
            rightMargin: 14
            topMargin: 4
            bottomMargin: 4
        }

    }

    MouseArea {
        id: hoverArea

        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: (mouse) => {
            if (mouse.button === Qt.RightButton)
                root.rightClicked(mouse);
            else
                root.clicked(mouse);
        }
        onWheel: (wheel) => {
            return root.scrolled(wheel.angleDelta.y > 0 ? 1 : -1);
        }
    }

    Behavior on color {
        ColorAnimation {
            duration: 300
            easing.type: Easing.OutCubic
        }

    }

}
