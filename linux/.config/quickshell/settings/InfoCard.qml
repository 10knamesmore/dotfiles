import QtQuick
import "../theme"

// 通用信息卡片 — hover 变色边框动画
Rectangle {
    id: root

    property Item contentItem: null

    onContentItemChanged: {
        if (contentItem) {
            contentItem.parent = cardInner;
            contentItem.anchors.left = cardInner.left;
            contentItem.anchors.right = cardInner.right;
        }
    }

    implicitHeight: contentItem ? contentItem.implicitHeight + 20 : 20
    radius: 10
    color: cardHover.containsMouse ? Colors.surface1 : Colors.surface0
    border.color: cardHover.containsMouse ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.2) : Qt.rgba(1, 1, 1, 0.04)
    border.width: 1

    Behavior on color {
        ColorAnimation {
            duration: 200
            easing.type: Easing.OutCubic
        }
    }
    Behavior on border.color {
        ColorAnimation {
            duration: 200
            easing.type: Easing.OutCubic
        }
    }

    Item {
        id: cardInner
        anchors.fill: parent
        anchors.margins: 10
    }

    MouseArea {
        id: cardHover
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.NoButton
    }
}
