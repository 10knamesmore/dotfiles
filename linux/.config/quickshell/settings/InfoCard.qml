import "../theme"
import QtQuick

// 通用信息卡片 — 毛玻璃 + hover 变色边框动画 + 微缩放
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
    radius: Tokens.radiusM
    color: cardHover.containsMouse ? Qt.rgba(Colors.surface1.r, Colors.surface1.g, Colors.surface1.b, Tokens.cardAlpha) : Qt.rgba(Colors.surface0.r, Colors.surface0.g, Colors.surface0.b, Tokens.cardAlpha)
    border.color: cardHover.containsMouse ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.25) : Qt.rgba(1, 1, 1, 0.06)
    border.width: 1
    scale: cardHover.containsMouse ? 1.01 : 1

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

    Behavior on scale {
        NumberAnimation {
            duration: 250
            easing.type: Easing.OutBack
        }

    }

}
