import "../theme"
import QtQuick

// 显示器排布画布：按相对位置画矩形，可拖拽改 x/y，松手吸附对齐其它屏与原点。
Rectangle {
    id: canvas

    property var monitors: []      // draft 条目数组
    property int selectedIndex: 0
    signal monitorSelected(int index)
    signal monitorMoved(int index, int x, int y)

    readonly property int _pad: 24
    readonly property int _snapThreshold: 60   // 逻辑像素

    radius: Tokens.radiusM
    color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, 0.5)
    border.width: 1
    border.color: Qt.rgba(1, 1, 1, 0.06)
    clip: true

    function _logW(m) { return Math.round(m.width / m.scale); }
    function _logH(m) { return Math.round(m.height / m.scale); }

    // 仅排布启用的屏（禁用的不占位）
    function _enabled() {
        return (monitors || []).filter(function (m) { return m.enabled; });
    }

    // ── 缩放/偏移：把所有屏装进画布并居中 ──
    property real _minX: 0
    property real _minY: 0
    property real _sf: 1
    property real _offX: 0
    property real _offY: 0

    // 只在「显示器集合 / 盒子尺寸 / 画布尺寸」变化时 refit；纯位置拖动不 refit，
    // 否则拖一个框会让整个画布重新居中缩放、所有框跳位（错位感）。
    property string _fitKey: ""
    function _maybeFit() {
        var en = _enabled();
        var key = en.map(function (m) { return m.name + ":" + _logW(m) + "x" + _logH(m); }).sort().join("|") + "@" + width + "x" + height;
        if (key === _fitKey)
            return;
        _fitKey = key;
        _recompute();
    }

    function _recompute() {
        var en = _enabled();
        if (en.length === 0)
            return;
        var minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
        en.forEach(function (m) {
            minX = Math.min(minX, m.x);
            minY = Math.min(minY, m.y);
            maxX = Math.max(maxX, m.x + _logW(m));
            maxY = Math.max(maxY, m.y + _logH(m));
        });
        var spanW = Math.max(1, maxX - minX);
        var spanH = Math.max(1, maxY - minY);
        var sf = Math.min((width - 2 * _pad) / spanW, (height - 2 * _pad) / spanH);
        if (!isFinite(sf) || sf <= 0)
            sf = 0.1;
        canvas._minX = minX;
        canvas._minY = minY;
        canvas._sf = sf;
        canvas._offX = (width - spanW * sf) / 2;
        canvas._offY = (height - spanH * sf) / 2;
    }

    function _toPxX(lx) { return _offX + (lx - _minX) * _sf; }
    function _toPxY(ly) { return _offY + (ly - _minY) * _sf; }
    function _toLogX(px) { return Math.round((px - _offX) / _sf + _minX); }
    function _toLogY(py) { return Math.round((py - _offY) / _sf + _minY); }

    // 松手吸附：对齐其它屏的边/角，及原点
    function _snap(index, lx, ly) {
        var me = monitors[index];
        var w = _logW(me), h = _logH(me);
        var t = _snapThreshold;
        for (var i = 0; i < monitors.length; i++) {
            if (i === index || !monitors[i].enabled)
                continue;
            var o = monitors[i], ow = _logW(o), oh = _logH(o);
            // 水平贴边
            if (Math.abs(lx - (o.x + ow)) < t) lx = o.x + ow;        // 我在它右边
            else if (Math.abs((lx + w) - o.x) < t) lx = o.x - w;     // 我在它左边
            else if (Math.abs(lx - o.x) < t) lx = o.x;               // 左边对齐
            else if (Math.abs((lx + w) - (o.x + ow)) < t) lx = o.x + ow - w; // 右边对齐
            // 垂直贴边
            if (Math.abs(ly - (o.y + oh)) < t) ly = o.y + oh;
            else if (Math.abs((ly + h) - o.y) < t) ly = o.y - h;
            else if (Math.abs(ly - o.y) < t) ly = o.y;
            else if (Math.abs((ly + h) - (o.y + oh)) < t) ly = o.y + oh - h;
        }
        return { "x": lx, "y": ly };
    }

    onWidthChanged: _maybeFit()
    onHeightChanged: _maybeFit()
    onMonitorsChanged: _maybeFit()

    Repeater {
        id: rep
        model: canvas.monitors

        delegate: Rectangle {
            id: tile
            property bool dragging: false
            property bool isSelected: index === canvas.selectedIndex

            width: Math.max(28, canvas._logW(modelData) * canvas._sf)
            height: Math.max(20, canvas._logH(modelData) * canvas._sf)
            radius: Tokens.radiusS
            visible: modelData.enabled
            opacity: modelData.enabled ? 1 : 0.4
            color: isSelected ? Qt.rgba(Colors.blue.r, Colors.blue.g, Colors.blue.b, 0.22) : Qt.rgba(Colors.surface1.r, Colors.surface1.g, Colors.surface1.b, 0.5)
            border.width: 2
            border.color: isSelected ? Colors.green : Colors.blue
            z: isSelected ? 2 : 1

            Binding on x { value: canvas._toPxX(modelData.x); when: !tile.dragging; restoreMode: Binding.RestoreBindingOrValue }
            Binding on y { value: canvas._toPxY(modelData.y); when: !tile.dragging; restoreMode: Binding.RestoreBindingOrValue }

            Column {
                anchors.centerIn: parent
                spacing: 1
                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: modelData.name
                    color: Colors.text
                    font.family: Fonts.family
                    font.pixelSize: Fonts.xs
                    font.bold: true
                }
                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: modelData.width + "×" + modelData.height
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.xs - 1
                }
            }

            // 主屏角标
            Rectangle {
                visible: modelData.primary
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.margins: 3
                width: pri.implicitWidth + 6
                height: pri.implicitHeight + 2
                radius: Tokens.radiusXS
                color: Colors.green
                Text { id: pri; anchors.centerIn: parent; text: "主"; color: Colors.base; font.family: Fonts.family; font.pixelSize: Fonts.xs - 1; font.bold: true }
            }

            MouseArea {
                anchors.fill: parent
                drag.target: parent
                drag.axis: Drag.XAndYAxis
                drag.minimumX: 0
                drag.maximumX: Math.max(0, canvas.width - tile.width)
                drag.minimumY: 0
                drag.maximumY: Math.max(0, canvas.height - tile.height)
                cursorShape: Qt.PointingHandCursor
                onPressed: {
                    canvas.monitorSelected(index);
                    tile.dragging = true;
                }
                onReleased: {
                    var lx = canvas._toLogX(tile.x);
                    var ly = canvas._toLogY(tile.y);
                    var s = canvas._snap(index, lx, ly);
                    tile.dragging = false;
                    canvas.monitorMoved(index, s.x, s.y);
                }
            }

            Behavior on color { ColorAnimation { duration: Tokens.animFast } }
            Behavior on border.color { ColorAnimation { duration: Tokens.animFast } }
        }
    }

    // 空态
    Text {
        anchors.centerIn: parent
        visible: canvas._enabled().length === 0
        text: "无启用的显示器"
        color: Colors.overlay0
        font.family: Fonts.family
        font.pixelSize: Fonts.small
    }
}
