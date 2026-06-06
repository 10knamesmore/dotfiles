import "../theme"
import QtQuick

// 多显示器布局缩略预览 — 按比例缩放显示各屏幕相对位置
Item {
    id: root

    property var monitors: []
    property int selectedIndex: 0
    signal monitorClicked(int index)

    implicitHeight: 150

    // 计算包围盒和缩放
    property var _layout: {
        if (!monitors || monitors.length === 0)
            return { "rects": [], "scale": 1 };

        let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
        for (let m of monitors) {
            let mx = m.x || 0, my = m.y || 0;
            let mw = m.width || 1920, mh = m.height || 1080;
            if (mx < minX) minX = mx;
            if (my < minY) minY = my;
            if (mx + mw > maxX) maxX = mx + mw;
            if (my + mh > maxY) maxY = my + mh;
        }

        let totalW = maxX - minX;
        let totalH = maxY - minY;
        let availW = root.width - 20;
        let availH = root.height - 20;
        let sc = Math.min(availW / totalW, availH / totalH, 1);

        let rects = [];
        for (let i = 0; i < monitors.length; i++) {
            let m = monitors[i];
            rects.push({
                "x": (m.x - minX) * sc + (availW - totalW * sc) / 2 + 10,
                "y": (m.y - minY) * sc + (availH - totalH * sc) / 2 + 10,
                "w": m.width * sc,
                "h": m.height * sc,
                "name": m.name || ("Monitor " + i)
            });
        }
        return { "rects": rects, "scale": sc };
    }

    Repeater {
        model: root._layout.rects.length

        delegate: Rectangle {
            id: monRect

            required property int index

            property var rect: root._layout.rects[index]

            x: rect.x
            y: rect.y
            width: rect.w
            height: rect.h
            radius: Tokens.radiusS
            color: index === root.selectedIndex
                ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, monHover.containsMouse ? 0.25 : 0.15)
                : (monHover.containsMouse ? Colors.surface1 : Qt.rgba(Colors.surface0.r, Colors.surface0.g, Colors.surface0.b, 0.8))
            border.color: index === root.selectedIndex
                ? Colors.blue
                : (monHover.containsMouse ? Colors.overlay0 : Qt.rgba(1, 1, 1, Tokens.borderAlpha))
            border.width: index === root.selectedIndex ? 2 : 1

            Text {
                anchors.centerIn: parent
                text: monRect.rect.name
                color: monRect.index === root.selectedIndex ? Colors.blue : Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.small
                font.weight: monRect.index === root.selectedIndex ? Font.Bold : Font.Normal
            }

            MouseArea {
                id: monHover
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.monitorClicked(monRect.index)
            }

            Behavior on color { ColorAnimation { duration: 150 } }
            Behavior on border.color { ColorAnimation { duration: 150 } }
        }
    }
}
