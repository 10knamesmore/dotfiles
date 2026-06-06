import "../components"
import "../services"
import "../theme"
import "../state"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Services.Mpris

// 媒体控制面板 — 使用 PanelOverlay 通用动画
PanelOverlay {
    id: root

    showing: PanelState.mediaOpen
    panelWidth: 520
    panelHeight: Math.min(height * 1.2, col.implicitHeight + 82)
    onCloseRequested: PanelState.mediaOpen = false
    onShowingChanged: {
        if (showing && LyricsState.currentLyricIndex >= 0)
            lyricsJumpTimer.start();
    }

    readonly property var player: MediaService.activePlayer

    function formatTime(seconds) {
        if (!seconds || seconds < 0)
            return "0:00";

        let m = Math.floor(seconds / 60);
        let s = Math.floor(seconds % 60);
        return m + ":" + (s < 10 ? "0" : "") + s;
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
            text: root.player ? (root.player.trackArtist || "") : ""
            visible: text.length > 0
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.body
            elide: Text.ElideRight
            horizontalAlignment: Text.AlignHCenter
        }

        Text {
            Layout.fillWidth: true
            text: root.player ? (root.player.trackAlbum || "") : ""
            visible: text.length > 0
            color: Colors.overlay0
            font.family: Fonts.family
            font.pixelSize: Fonts.small
            font.italic: true
            elide: Text.ElideRight
            horizontalAlignment: Text.AlignHCenter
        }

        // ── 歌词偏移调节 ──
        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: Tokens.spaceS
            visible: LyricsState.lyricsLines.length > 0

            Rectangle {
                width: 28
                height: 28
                radius: Tokens.radiusFull
                color: offMinusArea.containsMouse ? Colors.surface1 : "transparent"
                Text {
                    anchors.centerIn: parent
                    text: "−"
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.body
                }
                MouseArea {
                    id: offMinusArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: LyricsState.lyricsOffset = Math.round((LyricsState.lyricsOffset - 0.1) * 10) / 10
                }
                Behavior on color {
                    ColorAnimation {
                        duration: Tokens.animFast
                    }
                }
            }

            Text {
                text: {
                    let v = LyricsState.lyricsOffset;
                    return (v >= 0 ? "+" : "") + v.toFixed(1) + "s";
                }
                color: LyricsState.lyricsOffset === 0 ? Colors.overlay0 : Colors.mauve
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
                horizontalAlignment: Text.AlignHCenter
                Layout.preferredWidth: 44

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: LyricsState.lyricsOffset = 0
                }
            }

            Rectangle {
                width: 28
                height: 28
                radius: Tokens.radiusFull
                color: offPlusArea.containsMouse ? Colors.surface1 : "transparent"
                Text {
                    anchors.centerIn: parent
                    text: "+"
                    color: Colors.subtext0
                    font.family: Fonts.family
                    font.pixelSize: Fonts.body
                }
                MouseArea {
                    id: offPlusArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: LyricsState.lyricsOffset = Math.round((LyricsState.lyricsOffset + 0.1) * 10) / 10
                }
                Behavior on color {
                    ColorAnimation {
                        duration: Tokens.animFast
                    }
                }
            }

            // 分隔（仅当有翻译/罗马音可切时显示）
            Rectangle {
                visible: LyricsState.hasTranslation || LyricsState.hasRomanization
                Layout.preferredWidth: 1
                Layout.preferredHeight: 16
                color: Colors.surface1
            }

            // 翻译开关
            MediaButton {
                text: "译"
                active: LyricsState.showTranslation
                visible: LyricsState.hasTranslation
                onClicked: LyricsState.showTranslation = !LyricsState.showTranslation
            }

            // 罗马音开关
            MediaButton {
                text: "音"
                active: LyricsState.showRomanization
                visible: LyricsState.hasRomanization
                onClicked: LyricsState.showRomanization = !LyricsState.showRomanization
            }
        }

        // ── 歌词 ──
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 400
            clip: true
            visible: LyricsState.lyricsLines.length > 0

            ListView {
                id: lyricsView
                anchors.fill: parent
                model: LyricsState.lyricsLines
                spacing: 14
                clip: true
                interactive: true
                highlightFollowsCurrentItem: true
                preferredHighlightBegin: height / 2 - 16
                preferredHighlightEnd: height / 2 + 16
                currentIndex: LyricsState.currentLyricIndex >= 0 ? LyricsState.currentLyricIndex : 0

                // 用户手动滚动后暂停自动跟随，resumeTimer 到点再平滑回正
                property bool autoFollow: true

                // 显式动画 contentY：Behavior/positionViewAtIndex 走 C++ setContentY 绕过 QML 拦截器，
                // 不会动画；这里用 positionViewAtIndex 算出居中目标后回退，再 NumberAnimation 真正滚过去。
                NumberAnimation {
                    id: scrollAnim
                    target: lyricsView
                    property: "contentY"
                    duration: 500
                    easing.type: Easing.OutQuart
                }
                function scrollToCurrent(animated) {
                    if (LyricsState.currentLyricIndex < 0)
                        return;
                    let before = contentY;
                    positionViewAtIndex(LyricsState.currentLyricIndex, ListView.Center);
                    let target = contentY; // positionViewAtIndex 已把 contentY 同步置为居中目标（含边界 clamp）
                    scrollAnim.stop();
                    if (animated && Math.abs(target - before) > 1) {
                        contentY = before; // 回退，由动画从 before 平滑到 target
                        scrollAnim.from = before;
                        scrollAnim.to = target;
                        scrollAnim.start();
                    }
                    // 非动画分支：positionViewAtIndex 已就位（开屏 snap）
                }

                onMovementStarted: {
                    autoFollow = false;
                    resumeTimer.stop();
                    scrollAnim.stop();
                }
                onMovementEnded: resumeTimer.restart()

                Connections {
                    target: LyricsState
                    function onCurrentLyricIndexChanged() {
                        if (lyricsView.autoFollow)
                            lyricsView.scrollToCurrent(true);
                    }
                }

                delegate: Item {
                    id: lineRoot

                    required property int index
                    required property var modelData

                    property bool isCurrent: index === LyricsState.currentLyricIndex
                    // 仅当前行 + 有逐字数据时启用逐字高亮，其余行用整行文本
                    property bool useWords: LyricsState.hasWords && isCurrent && modelData.words && modelData.words.length > 0

                    width: ListView.view.width
                    implicitHeight: lineCol.implicitHeight
                    opacity: {
                        let dist = Math.abs(index - LyricsState.currentLyricIndex);
                        if (dist === 0)
                            return 1.0;
                        if (dist === 1)
                            return 0.72;
                        if (dist === 2)
                            return 0.5;
                        return 0.35;
                    }

                    Behavior on opacity {
                        NumberAnimation {
                            duration: 350
                            easing.type: Easing.OutCubic
                        }
                    }

                    // 测字宽用（与逐字文字同字体），供贪心分行
                    FontMetrics {
                        id: wordFm
                        font.family: Fonts.family
                        font.pixelSize: Fonts.h3
                        font.weight: Font.Bold
                    }

                    ColumnLayout {
                        id: lineCol
                        anchors.left: parent.left
                        anchors.right: parent.right
                        spacing: 3

                        // 原文（逐字高亮）— 超宽按字宽贪心换行，每视觉行居中
                        Column {
                            id: wordBlock
                            Layout.fillWidth: true
                            spacing: 2
                            visible: lineRoot.useWords

                            // 贪心分视觉行：[[word,...], ...]，累计字宽超 maxW 就换行
                            property var wordRows: {
                                if (!lineRoot.useWords)
                                    return [];
                                let maxW = lineRoot.width - 16;
                                let rows = [], cur = [], curW = 0;
                                for (let w of lineRoot.modelData.words) {
                                    let ww = wordFm.advanceWidth(w.text);
                                    if (curW + ww > maxW && cur.length > 0) {
                                        rows.push(cur);
                                        cur = [];
                                        curW = 0;
                                    }
                                    cur.push(w);
                                    curW += ww;
                                }
                                if (cur.length > 0)
                                    rows.push(cur);
                                return rows;
                            }

                            Repeater {
                                model: wordBlock.wordRows

                                delegate: Row {
                                    required property var modelData // 一视觉行：字数组
                                    anchors.horizontalCenter: parent.horizontalCenter
                                    spacing: 0

                                    Repeater {
                                        model: parent.modelData

                                        delegate: Text {
                                            required property var modelData // 单个字

                                            text: modelData.text
                                            font.family: Fonts.family
                                            font.pixelSize: Fonts.h3
                                            font.weight: Font.Bold
                                            color: {
                                                let wEnd = modelData.start + modelData.duration;
                                                if (LyricsState.currentTimeMs >= wEnd)
                                                    return Colors.text;          // 已唱
                                                if (LyricsState.currentTimeMs >= modelData.start)
                                                    return Colors.pink;          // 正在唱
                                                return Colors.overlay1;          // 未唱
                                            }

                                            Behavior on color {
                                                ColorAnimation {
                                                    duration: 150
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // 原文（整行）— 非逐字行 / 非当前行
                        Text {
                            visible: !lineRoot.useWords
                            Layout.fillWidth: true
                            text: lineRoot.modelData.text
                            color: lineRoot.isCurrent ? Colors.text : Colors.subtext0
                            font.family: Fonts.family
                            font.pixelSize: lineRoot.isCurrent ? Fonts.h3 : Fonts.heading
                            font.weight: lineRoot.isCurrent ? Font.Bold : Font.Normal
                            wrapMode: Text.WordWrap
                            horizontalAlignment: Text.AlignHCenter

                            Behavior on color {
                                ColorAnimation {
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

                        // 翻译
                        Text {
                            visible: LyricsState.showTranslation && lineRoot.modelData.translation && lineRoot.modelData.translation.length > 0
                            Layout.fillWidth: true
                            text: lineRoot.modelData.translation
                            color: lineRoot.isCurrent ? Colors.subtext0 : Colors.overlay2
                            font.family: Fonts.family
                            font.pixelSize: Fonts.body
                            wrapMode: Text.WordWrap
                            horizontalAlignment: Text.AlignHCenter
                        }

                        // 罗马音
                        Text {
                            visible: LyricsState.showRomanization && lineRoot.modelData.romanization && lineRoot.modelData.romanization.length > 0
                            Layout.fillWidth: true
                            text: lineRoot.modelData.romanization
                            color: lineRoot.isCurrent ? Colors.subtext0 : Colors.overlay1
                            font.family: Fonts.family
                            font.pixelSize: Fonts.caption
                            font.italic: true
                            wrapMode: Text.WordWrap
                            horizontalAlignment: Text.AlignHCenter
                        }
                    }
                }

            }

            // 开屏瞬时定位到当前行（不走动画，避免从 0 长距离滚动）
            Timer {
                id: lyricsJumpTimer
                interval: 50
                onTriggered: lyricsView.scrollToCurrent(false)
            }

            // 用户停手 4s 后恢复跟随并平滑滚回当前行
            Timer {
                id: resumeTimer
                interval: 4000
                onTriggered: {
                    lyricsView.autoFollow = true;
                    lyricsView.scrollToCurrent(true);
                }
            }
        }

        // ── 无歌词占位 ──
        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 60
            visible: LyricsState.lyricsLines.length === 0

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
            id: progressItem
            Layout.fillWidth: true
            Layout.preferredHeight: 24
            visible: root.player && root.player.lengthSupported

            // 播放进度比例（0~1），fill 与 knob 共用
            readonly property real fillRatio: (root.player && root.player.length > 0) ? Math.min(1, root.player.position / root.player.length) : 0

            Rectangle {
                id: progressBg

                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                height: progressArea.containsMouse || progressArea.pressed ? 8 : 4
                radius: height / 2
                color: Colors.surface1

                Behavior on height {
                    NumberAnimation {
                        duration: Tokens.animFast
                        easing.type: Easing.OutCubic
                    }
                }

                Rectangle {
                    width: progressItem.fillRatio * parent.width
                    height: parent.height
                    radius: parent.radius
                    color: Colors.pink

                    // 拖动时即时跟手；seek 长跳 / 播放推进用 OutCubic 减速收尾，避免线性匀速生硬
                    Behavior on width {
                        enabled: !progressArea.pressed
                        NumberAnimation {
                            duration: 500
                            easing.type: Easing.OutCubic
                        }
                    }
                }

                // 拖动手柄：hover/按下时浮现
                Rectangle {
                    x: progressItem.fillRatio * parent.width - width / 2
                    anchors.verticalCenter: parent.verticalCenter
                    width: progressArea.containsMouse || progressArea.pressed ? 14 : 0
                    height: width
                    radius: width / 2
                    color: Colors.pink
                    opacity: progressArea.containsMouse || progressArea.pressed ? 1 : 0

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
            }

            // 点击/拖动进度条 seek（写 player.position，setter 自发 positionChanged）
            MouseArea {
                id: progressArea
                anchors.fill: progressBg
                anchors.topMargin: -8
                anchors.bottomMargin: -8
                enabled: root.player && root.player.canSeek && root.player.length > 0
                hoverEnabled: true
                cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor

                function seekTo(mouseX) {
                    if (!root.player || root.player.length <= 0)
                        return;
                    let ratio = Math.max(0, Math.min(1, mouseX / width));
                    root.player.position = ratio * root.player.length;
                }
                onClicked: mouse => seekTo(mouse.x)
                onPositionChanged: mouse => {
                    if (pressed)
                        seekTo(mouse.x);
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

            // Shuffle — 原生 setShuffle，发 MPRIS D-Bus Set；active 经 shuffleChanged 实时回显
            MediaButton {
                text: "󰒟"
                active: root.player && root.player.shuffle
                visible: root.player && root.player.shuffleSupported
                onClicked: {
                    if (root.player)
                        root.player.shuffle = !root.player.shuffle;
                }
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
                // 原生 setLoopState 循环 None→Playlist→Track；图标经 loopStateChanged 实时回显
                onClicked: {
                    if (!root.player)
                        return;

                    switch (root.player.loopState) {
                    case MprisLoopState.None:
                        root.player.loopState = MprisLoopState.Playlist;
                        break;
                    case MprisLoopState.Playlist:
                        root.player.loopState = MprisLoopState.Track;
                        break;
                    default:
                        root.player.loopState = MprisLoopState.None;
                        break;
                    }
                }
            }
        }

        // ── 系统音量滑块（AudioService）──
        RowLayout {
            id: volRow
            Layout.fillWidth: true
            spacing: Tokens.spaceS

            property int volPct: AudioService.volume
            property bool muted: AudioService.muted

            Text {
                text: volRow.muted ? "󰝟" : (volRow.volPct < 50 ? "󰖀" : "󰕾")
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.icon

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: AudioService.toggleMute()
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
                        AudioService.setVolume(pct);
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
