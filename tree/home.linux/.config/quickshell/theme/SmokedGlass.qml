import QtQuick

// 面板的静态底色与细颗粒。背景模糊交给 compositor，不采样其他窗口。
Item {
    id: root

    property real radius: Tokens.radiusL
    property real fillOpacity: 0
    property color tint: Colors.base

    Accessible.ignored: true

    Rectangle {
        anchors.fill: parent
        radius: root.radius
        color: Colors.withAlpha(root.tint, root.fillOpacity)
    }

    Image {
        anchors.fill: parent
        // 留出圆角的弧边，让纹理始终落在宿主的圆角内。
        anchors.margins: Math.ceil(root.radius * 0.3)
        source: "grain.png"
        fillMode: Image.Tile
        opacity: 0.085
    }
}
