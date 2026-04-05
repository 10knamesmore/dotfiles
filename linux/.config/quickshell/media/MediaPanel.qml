import "../theme"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Services.Mpris
import Quickshell.Wayland

// 媒体控制面板 — 显示封面、曲目信息、进度条、控制按钮
PanelWindow {
    id: root

    // 双阶段可见性
    property bool showing: PanelState.mediaOpen
    property bool animating: _opacityAnim.running || _slideAnim.running
    property var player: {
        let ps = Mpris.players.values;
        for (let i = 0; i < ps.length; i++) {
            if (ps[i].isPlaying) {
                PanelState.lastActivePlayer = ps[i];
                return ps[i];
            }
        }
        if (PanelState.lastActivePlayer && ps.indexOf(PanelState.lastActivePlayer) >= 0)
            return PanelState.lastActivePlayer;
        return ps.length > 0 ? ps[0] : null;
    }

    function formatTime(seconds) {
        if (!seconds || seconds < 0)
            return "0:00";

        let m = Math.floor(seconds / 60);
        let s = Math.floor(seconds % 60);
        return m + ":" + (s < 10 ? "0" : "") + s;
    }

    anchors.top: true
    anchors.bottom: true
    anchors.left: true
    anchors.right: true
    visible: showing || animating
    focusable: false
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"
    onShowingChanged: {
        if (showing && PanelState.currentLyricIndex >= 0)
            lyricsJumpTimer.start();
    }

    // 半透明遮罩
    Rectangle {
        anchors.fill: parent
        color: "#000000"
        opacity: root.showing ? Tokens.backdropDim : 0

        Behavior on opacity {
            NumberAnimation {
                duration: Tokens.animNormal
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Anim.standard
            }
        }
    }

    // 点击面板外部关闭
    MouseArea {
        anchors.fill: parent
        onClicked: PanelState.mediaOpen = false
    }

    Rectangle {
        id: panel

        width: 400
        height: Math.min(root.height * 0.85, col.implicitHeight + 32)
        radius: Tokens.radiusL
        color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, Tokens.panelAlpha)
        border.color: Qt.rgba(1, 1, 1, Tokens.borderAlpha)
        border.width: 1
        anchors.centerIn: parent
        anchors.verticalCenterOffset: root.showing ? 0 : 20
        opacity: root.showing ? 1 : 0

        SoftShadow {
            anchors.fill: parent
            radius: parent.radius
        }

        MouseArea {
            anchors.fill: parent
            onClicked: mouse => {
                return mouse.accepted = true;
            }
        }

        ColumnLayout {
            id: col

            anchors.fill: parent
            anchors.margins: Tokens.spaceL
            spacing: 10

            // ── 封面 ──
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: width
                radius: Tokens.radiusM
                color: Colors.surface1
                clip: true

                Image {
                    id: coverArt

                    anchors.fill: parent
                    source: root.player && root.player.trackArtUrl ? root.player.trackArtUrl : ""
                    fillMode: Image.PreserveAspectFit
                    visible: status === Image.Ready
                }

                Text {
                    anchors.centerIn: parent
                    text: "󰎆"
                    font.family: Fonts.family
                    font.pixelSize: Fonts.display1
                    color: Colors.overlay1
                    visible: !coverArt.visible
                }
            }

            // ── 曲目信息 ──
            Text {
                Layout.fillWidth: true
                text: root.player ? (root.player.trackTitle || "未在播放") : "无播放器"
                color: Colors.text
                font.family: Fonts.family
                font.pixelSize: Fonts.icon
                font.weight: Font.Bold
                elide: Text.ElideRight
                horizontalAlignment: Text.AlignHCenter
            }

            Text {
                Layout.fillWidth: true
                text: {
                    if (!root.player)
                        return "";

                    let parts = [];
                    if (root.player.trackArtist)
                        parts.push(root.player.trackArtist);

                    if (root.player.trackAlbum)
                        parts.push(root.player.trackAlbum);

                    return parts.join(" — ");
                }
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.small
                elide: Text.ElideRight
                horizontalAlignment: Text.AlignHCenter
            }

            // ── 歌词偏移调节 ──
            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: Tokens.spaceS
                visible: PanelState.lyricsLines.length > 0

                Rectangle {
                    width: 28; height: 28; radius: Tokens.radiusFull
                    color: offMinusArea.containsMouse ? Colors.surface1 : "transparent"
                    Text {
                        anchors.centerIn: parent; text: "−"
                        color: Colors.subtext0; font.family: Fonts.family; font.pixelSize: Fonts.body
                    }
                    MouseArea {
                        id: offMinusArea; anchors.fill: parent; hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: PanelState.lyricsOffset = Math.round((PanelState.lyricsOffset - 0.1) * 10) / 10
                    }
                    Behavior on color { ColorAnimation { duration: Tokens.animFast } }
                }

                Text {
                    text: {
                        let v = PanelState.lyricsOffset;
                        return (v >= 0 ? "+" : "") + v.toFixed(1) + "s";
                    }
                    color: PanelState.lyricsOffset === 0 ? Colors.overlay0 : Colors.mauve
                    font.family: Fonts.family
                    font.pixelSize: Fonts.caption
                    horizontalAlignment: Text.AlignHCenter
                    Layout.preferredWidth: 44

                    MouseArea {
                        anchors.fill: parent; cursorShape: Qt.PointingHandCursor
                        onClicked: PanelState.lyricsOffset = 0
                    }
                }

                Rectangle {
                    width: 28; height: 28; radius: Tokens.radiusFull
                    color: offPlusArea.containsMouse ? Colors.surface1 : "transparent"
                    Text {
                        anchors.centerIn: parent; text: "+"
                        color: Colors.subtext0; font.family: Fonts.family; font.pixelSize: Fonts.body
                    }
                    MouseArea {
                        id: offPlusArea; anchors.fill: parent; hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: PanelState.lyricsOffset = Math.round((PanelState.lyricsOffset + 0.1) * 10) / 10
                    }
                    Behavior on color { ColorAnimation { duration: Tokens.animFast } }
                }
            }

            // ── 歌词 ──
            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumHeight: 400
                clip: true
                visible: PanelState.lyricsLines.length > 0

                ListView {
                    id: lyricsView
                    anchors.fill: parent
                    model: PanelState.lyricsLines
                    spacing: 14
                    clip: true
                    interactive: true
                    highlightFollowsCurrentItem: true
                    preferredHighlightBegin: height / 2 - 16
                    preferredHighlightEnd: height / 2 + 16
                    highlightRangeMode: ListView.ApplyRange
                    currentIndex: PanelState.currentLyricIndex >= 0 ? PanelState.currentLyricIndex : 0

                    delegate: Text {
                        required property int index
                        required property var modelData

                        property bool isCurrent: index === PanelState.currentLyricIndex

                        width: ListView.view.width
                        text: modelData.text
                        color: isCurrent ? Colors.text : Colors.overlay0
                        font.family: Fonts.family
                        font.pixelSize: isCurrent ? Fonts.h3 : Fonts.heading
                        font.weight: isCurrent ? Font.Bold : Font.Normal
                        wrapMode: Text.WordWrap
                        horizontalAlignment: Text.AlignCenter
                        opacity: {
                            let dist = Math.abs(index - PanelState.currentLyricIndex);
                            if (dist === 0)
                                return 1.0;
                            if (dist === 1)
                                return 0.5;
                            if (dist === 2)
                                return 0.25;
                            return 0.12;
                        }

                        Behavior on color {
                            ColorAnimation {
                                duration: 350
                                easing.type: Easing.OutCubic
                            }
                        }
                        Behavior on opacity {
                            NumberAnimation {
                                duration: 350
                                easing.type: Easing.OutCubic
                            }
                        }
                        Behavior on font.pixelSize {
                            NumberAnimation {
                                duration: 350
                                easing.type: Easing.OutCubic
                            }
                        }
                    }

                    Behavior on contentY {
                        enabled: !lyricsJumpTimer.running
                        NumberAnimation {
                            duration: 500
                            easing.type: Easing.OutQuart
                        }
                    }
                }

                Timer {
                    id: lyricsJumpTimer
                    interval: 50
                    onTriggered: {
                        if (PanelState.currentLyricIndex >= 0)
                            lyricsView.positionViewAtIndex(PanelState.currentLyricIndex, ListView.Center);
                    }
                }
            }

            // ── 无歌词占位 ──
            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: 60
                visible: PanelState.lyricsLines.length === 0

                Text {
                    anchors.centerIn: parent
                    text: "暂无歌词"
                    color: Colors.overlay0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.body
                    font.italic: true
                }
            }

            // ── 进度条 ──
            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: 24
                visible: root.player && root.player.lengthSupported

                Rectangle {
                    id: progressBg

                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    height: 4
                    radius: 2
                    color: Colors.surface1

                    Rectangle {
                        width: {
                            if (!root.player || root.player.length <= 0)
                                return 0;

                            return Math.min(1, root.player.position / root.player.length) * parent.width;
                        }
                        height: parent.height
                        radius: parent.radius
                        color: Colors.pink

                        Behavior on width {
                            NumberAnimation {
                                duration: 500
                            }
                        }
                    }
                }

                // 进度时间
                RowLayout {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: progressBg.bottom
                    anchors.topMargin: 2

                    Text {
                        text: root.player ? formatTime(root.player.position) : "0:00"
                        color: Colors.subtext0
                        font.family: Fonts.family
                        font.pixelSize: Fonts.caption
                    }

                    Item {
                        Layout.fillWidth: true
                    }

                    Text {
                        text: root.player ? formatTime(root.player.length) : "0:00"
                        color: Colors.subtext0
                        font.family: Fonts.family
                        font.pixelSize: Fonts.caption
                    }
                }
            }

            // ── 控制按钮 ──
            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 20

                // Shuffle
                MediaButton {
                    text: "󰒟"
                    active: root.player && root.player.shuffle
                    visible: root.player && root.player.shuffleSupported
                    onClicked: {
                        if (root.player) {
                            let val = root.player.shuffle ? "Off" : "On";
                            shuffleProc.command = ["playerctl", "-p", root.player.identity, "shuffle", val];
                            shuffleProc.running = true;
                        }
                    }
                }

                Process {
                    id: shuffleProc
                }

                // Previous
                MediaButton {
                    text: "󰒮"
                    enabled: root.player && root.player.canGoPrevious
                    onClicked: {
                        if (root.player)
                            root.player.previous();
                    }
                }

                // Play/Pause (大按钮)
                Rectangle {
                    width: 48
                    height: 48
                    radius: 24
                    color: playPauseArea.containsMouse ? Colors.pink : Colors.surface1

                    Text {
                        anchors.centerIn: parent
                        text: root.player && root.player.isPlaying ? "󰏤" : "󰐊"
                        color: playPauseArea.containsMouse ? Colors.base : Colors.text
                        font.family: Fonts.family
                        font.pixelSize: Fonts.h2
                    }

                    MouseArea {
                        id: playPauseArea

                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (root.player)
                                root.player.togglePlaying();
                        }
                    }

                    Behavior on color {
                        ColorAnimation {
                            duration: Tokens.animFast
                            easing.type: Easing.BezierSpline
                            easing.bezierCurve: Anim.standard
                        }
                    }
                }

                // Next
                MediaButton {
                    text: "󰒭"
                    enabled: root.player && root.player.canGoNext
                    onClicked: {
                        if (root.player)
                            root.player.next();
                    }
                }

                // Loop
                MediaButton {
                    text: {
                        if (!root.player)
                            return "󰑗";

                        switch (root.player.loopState) {
                        case MprisLoopState.Track:
                            return "󰑘";
                        case MprisLoopState.Playlist:
                            return "󰑖";
                        default:
                            return "󰑗";
                        }
                    }
                    active: root.player && root.player.loopState !== MprisLoopState.None
                    visible: root.player && root.player.loopSupported
                    onClicked: {
                        if (!root.player)
                            return;

                        let val;
                        switch (root.player.loopState) {
                        case MprisLoopState.None:
                            val = "Playlist";
                            break;
                        case MprisLoopState.Playlist:
                            val = "Track";
                            break;
                        default:
                            val = "None";
                            break;
                        }
                        loopProc.command = ["playerctl", "-p", root.player.identity, "loop", val];
                        loopProc.running = true;
                    }
                }

                Process {
                    id: loopProc
                }
            }

            // ── 系统音量滑块（wpctl）──
            RowLayout {
                id: volRow
                Layout.fillWidth: true
                spacing: Tokens.spaceS

                property int volPct: 0
                property bool muted: false

                Process {
                    id: volReader
                    command: ["wpctl", "get-volume", "@DEFAULT_AUDIO_SINK@"]
                    stdout: SplitParser {
                        onRead: data => {
                            let m = data.match(/Volume:\s+([\d.]+)(\s+\[MUTED\])?/);
                            if (m) {
                                volRow.volPct = Math.round(parseFloat(m[1]) * 100);
                                volRow.muted = m[2] !== undefined;
                            }
                        }
                    }
                }

                Timer {
                    running: root.showing
                    interval: 16
                    repeat: true
                    triggeredOnStart: true
                    onTriggered: volReader.running = true
                }

                Process {
                    id: volSetProc
                }
                Process {
                    id: muteProc
                    command: ["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]
                }

                Text {
                    text: volRow.muted ? "󰝟" : (volRow.volPct < 50 ? "󰖀" : "󰕾")
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.icon

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: muteProc.running = true
                    }
                }

                Rectangle {
                    id: volSlider
                    Layout.fillWidth: true
                    height: volSliderArea.containsMouse || volSliderArea.pressed ? 8 : 4
                    radius: height / 2
                    color: Colors.surface1

                    Behavior on height {
                        NumberAnimation {
                            duration: Tokens.animFast
                            easing.type: Easing.OutCubic
                        }
                    }

                    Rectangle {
                        width: volRow.volPct / 100 * parent.width
                        height: parent.height
                        radius: parent.radius
                        color: Colors.pink

                        Behavior on width {
                            enabled: !volSliderArea.pressed
                            NumberAnimation {
                                duration: 100
                            }
                        }
                    }

                    Rectangle {
                        x: volRow.volPct / 100 * parent.width - width / 2
                        anchors.verticalCenter: parent.verticalCenter
                        width: volSliderArea.containsMouse || volSliderArea.pressed ? 14 : 0
                        height: width
                        radius: width / 2
                        color: Colors.pink
                        opacity: volSliderArea.containsMouse || volSliderArea.pressed ? 1 : 0

                        Behavior on width {
                            NumberAnimation {
                                duration: Tokens.animFast
                                easing.type: Easing.OutCubic
                            }
                        }
                        Behavior on opacity {
                            NumberAnimation {
                                duration: Tokens.animFast
                            }
                        }
                    }

                    MouseArea {
                        id: volSliderArea
                        anchors.fill: parent
                        anchors.margins: -6
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor

                        function setVol(mouseX) {
                            let pct = Math.max(0, Math.min(100, Math.round((mouseX - 6) / volSlider.width * 100)));
                            volSetProc.command = ["wpctl", "set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", pct + "%"];
                            volSetProc.running = true;
                        }

                        onClicked: mouse => setVol(mouse.x)
                        onPositionChanged: mouse => {
                            if (pressed)
                                setVol(mouse.x);
                        }
                    }
                }

                Text {
                    text: volRow.volPct + "%"
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.caption
                    Layout.preferredWidth: 32
                    horizontalAlignment: Text.AlignRight
                }
            }
        }

        InnerGlow {}

        Behavior on anchors.verticalCenterOffset {
            NumberAnimation {
                id: _slideAnim

                duration: Tokens.animSlow
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Anim.decelerate
            }
        }

        Behavior on opacity {
            NumberAnimation {
                id: _opacityAnim

                duration: Tokens.animNormal
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Anim.standard
            }
        }
    }

    // ── 位置更新定时器 ──
    Timer {
        interval: 1000
        running: root.visible && root.player && root.player.isPlaying
        repeat: true
        onTriggered: {} // 触发 position 重新绑定
    }

    // 可复用的小控制按钮
    component MediaButton: Rectangle {
        property string text: ""
        property bool active: false

        signal clicked

        width: 32
        height: 32
        radius: Tokens.radiusL
        color: btnArea.containsMouse ? Colors.surface1 : "transparent"

        Text {
            anchors.centerIn: parent
            text: parent.text
            color: parent.active ? Colors.pink : Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.heading
        }

        MouseArea {
            id: btnArea

            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: parent.clicked()
        }

        Behavior on color {
            ColorAnimation {
                duration: Tokens.animFast
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Anim.standard
            }
        }
    }
}
