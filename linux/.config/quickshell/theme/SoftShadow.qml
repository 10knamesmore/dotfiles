import QtQuick

// 柔和阴影 — 3 层递减透明度矩形模拟高斯扩散
// 用法：SoftShadow { anchors.fill: parent; radius: 16; shadowColor: Colors.blue; strength: 0.2 }
//
// 性能：3 层半透明 Rectangle 叠加每帧都要 blend，开启 layer 缓存让 GPU 只在
// strength / 几何 / radius 变化时重画一次到 FBO，平时合成只是单个纹理 quad。
Item {
    id: root

    property real radius: Tokens.radiusL
    property color shadowColor: "#000000"
    property real strength: Tokens.shadowOpacity

    z: -1
    layer.enabled: true
    layer.smooth: true

    // 第 1 层：最外圈，最淡
    Rectangle {
        anchors.fill: parent
        anchors.margins: -5
        anchors.bottomMargin: -7
        radius: root.radius + 5
        color: root.shadowColor
        opacity: root.strength * 0.25
    }

    // 第 2 层：中间
    Rectangle {
        anchors.fill: parent
        anchors.margins: -3
        anchors.bottomMargin: -5
        radius: root.radius + 3
        color: root.shadowColor
        opacity: root.strength * 0.5
    }

    // 第 3 层：最内圈，最浓
    Rectangle {
        anchors.fill: parent
        anchors.margins: -1
        anchors.bottomMargin: -3
        radius: root.radius + 2
        color: root.shadowColor
        opacity: root.strength
    }

    Behavior on strength {
        NumberAnimation {
            duration: Tokens.animNormal
            easing.type: Easing.OutCubic
        }

    }

}
