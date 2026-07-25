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
    color: cardHover.containsMouse ? Colors.withAlpha(Colors.surface1, Tokens.cardAlpha) : Colors.withAlpha(Colors.surface0, Tokens.cardAlpha)
    border.color: cardHover.containsMouse ? Colors.withAlpha(Colors.blue, Tokens.borderHoverAlpha) : Colors.overlay(0.06)
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
            duration: Tokens.animFast
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Anim.standard
        }

    }

    Behavior on border.color {
        ColorAnimation {
            duration: Tokens.animFast
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Anim.standard
        }

    }

    Behavior on scale {
        NumberAnimation {
            duration: Tokens.animNormal
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Anim.elastic
        }

    }

}
