import "../theme"
import "../state"
import QtQuick
import Quickshell
import Quickshell.Hyprland
import Quickshell.Wayland

// 顶栏展开时直接接管原模块的头部，文字、图标和进度保持实时更新。
// 一个进度同时驱动轮廓、补充信息和内容显现，收起直接回到顶栏模块。
PanelWindow {
    id: root

    property bool showing: false
    property real backdropOpacity: 0
    property real panelWidth: 400
    property real panelHeight: 400
    property int panelRadius: Tokens.radiusL
    property real panelTargetX: -1
    property real panelTargetY: -1
    property real closedOffsetX: 0
    property real closedOffsetY: 20
    // 控制中心的头部含独立按钮，关闭热区只占最右侧；其他模块仍可整头点击收起。
    property bool interactiveHeader: false
    property bool alignRight: false

    enum Entrance {
        Morph,
        Slide
    }
    property int entrance: PanelOverlay.Morph
    property var _source: null
    property rect _returnBounds: Qt.rect(0, 0, 0, 0)
    property bool _sourceHeld: false
    property real _closingDetails: 0
    property real _closingProgress: 1
    property real _openingDetails: 0
    property real _openingProgress: 0
    readonly property bool hasMorphSource: _source !== null
    default property alias panelContent: panelInner.data
    readonly property alias panel: panel

    property bool _keepVisible: false
    property bool _atTarget: false
    property bool _waitingForSize: false
    readonly property bool _sizeReady: root.width > 0 && root.height > 0 && (hasMorphSource ? root.width === _source.screen.width && root.height === _source.screen.height : _matchesAnyScreen(root.width, root.height))

    signal closeRequested

    function _matchesAnyScreen(w, h) {
        for (const screen of Quickshell.screens) {
            if (screen.width === w && screen.height === h)
                return true;
        }
        return false;
    }

    function _tryStartOpen() {
        if (!_waitingForSize || !_sizeReady)
            return;
        _waitingForSize = false;
        openTimer.start();
    }

    function _releaseSource() {
        if (!_sourceHeld)
            return;
        _source.header.parent = _source.home;
        MorphState.release(_source.item);
        _sourceHeld = false;
    }

    function _finishClose() {
        _keepVisible = false;
        if (_sourceHeld)
            console.info("[panel-morph] returned to bar module on", _source.screen.name);
        _releaseSource();
        _source = null;
    }

    function _animate(opening) {
        motion.stop();
        motion.from = panel.progress;
        motion.to = opening ? 1 : 0;
        motion.duration = (hasMorphSource ? (opening ? 520 : 200) : Tokens.animElaborate) * Math.abs(motion.to - motion.from);
        _atTarget = opening;
        motion.start();
    }

    onWidthChanged: _tryStartOpen()
    onHeightChanged: _tryStartOpen()
    onShowingChanged: {
        openTimer.stop();
        if (showing) {
            hideTimer.stop();
            const source = MorphState.takeSource();
            // 快速重新打开正在收起的面板时，沿当前轮廓反向展开。
            if (_sourceHeld && !source) {
                _openingDetails = _source.item.expansion;
                _openingProgress = panel.progress;
                _animate(true);
                return;
            }
            motion.stop();
            _releaseSource();
            _source = entrance === PanelOverlay.Morph ? source : null;
            if (hasMorphSource) {
                _returnBounds = _source.bounds;
                _openingDetails = _source.initialDetails;
                _openingProgress = 0;
            }
            screen = hasMorphSource ? _source.screen : null;
            panel.progress = 0;
            _atTarget = false;
            _keepVisible = true;
            contentViewport.contentY = 0;
            _waitingForSize = true;
            _tryStartOpen();
        } else {
            _waitingForSize = false;
            if (hasMorphSource) {
                // 曲目或计数变化可能改变顶栏宽度，终点使用当前顶栏模块的几何。
                _returnBounds = MorphState.boundsFor(_source.item, _source.barWindow);
                _closingDetails = _source.item.expansion;
                _closingProgress = panel.progress;
                if (_closingProgress > 0)
                    _animate(false);
                else
                    _finishClose();
            } else {
                _animate(false);
                hideTimer.start();
            }
        }
    }

    Component.onDestruction: _releaseSource()

    Timer {
        id: openTimer
        interval: 0
        onTriggered: {
            if (root.hasMorphSource) {
                root._source.header.parent = headerDock;
                MorphState.hold(root._source.item);
                root._sourceHeld = true;
                console.info("[panel-morph] live header on", root.screen.name, panel.targetX, panel.targetY, root.panelWidth, panel.targetHeight);
            }
            root._animate(true);
        }
    }

    NumberAnimation {
        id: motion
        target: panel
        property: "progress"
        easing.type: root.hasMorphSource ? (root._atTarget ? Easing.OutBack : Easing.OutCubic) : Easing.OutQuint
        easing.overshoot: 0.9
        onFinished: {
            if (!root.showing && root.hasMorphSource)
                root._finishClose();
        }
    }

    Timer {
        id: hideTimer
        interval: Tokens.animNormal + 50
        onTriggered: root._finishClose()
    }

    Binding {
        target: root._source ? root._source.item : null
        property: "expansion"
        value: Math.max(0, Math.min(1, !root.hasMorphSource ? 0 : root._atTarget ? (root._openingProgress < 1 ? root._openingDetails + (1 - root._openingDetails) * (panel.progress - root._openingProgress) / (1 - root._openingProgress) : 1) : (root._closingProgress > 0 ? root._closingDetails * panel.progress / root._closingProgress : 0)))
        when: root.hasMorphSource
    }

    Binding {
        target: root._source ? root._source.item : null
        property: "panelProgress"
        value: panel.revealProgress
        when: root.hasMorphSource
    }

    HyprlandFocusGrab {
        windows: [root]
        active: root.showing
    }

    anchors.top: true
    anchors.bottom: true
    anchors.left: true
    anchors.right: true
    visible: showing || _keepVisible
    focusable: showing
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "quickshell-panel"
    color: "transparent"

    // 只模糊面板当前的圆角轮廓；全屏遮罩与点击区域不参与模糊。
    BackgroundEffect.blurRegion: Region {
        item: panel
        radius: panel.radius
    }

    Rectangle {
        anchors.fill: parent
        color: "#000000"
        opacity: root._atTarget && !root.hasMorphSource ? root.backdropOpacity : 0
        Behavior on opacity {
            NumberAnimation {
                duration: Tokens.animNormal
                easing.type: Easing.OutCubic
            }
        }
    }

    Item {
        focus: root.showing
        Keys.onEscapePressed: root.closeRequested()
    }
    MouseArea {
        anchors.fill: parent
        onClicked: root.closeRequested()
    }

    Rectangle {
        id: panel

        property real progress: 0
        readonly property real revealProgress: Math.max(0, Math.min(1, progress))
        // 横向略微领先，纵向的回弹稍明显；两轴从第一帧同时运动。
        readonly property real widthProgress: root.hasMorphSource ? progress + 0.14 * Math.sin(Math.PI * progress) : progress
        readonly property real headerHeight: root.hasMorphSource ? root._source.bounds.height : 0
        readonly property real targetHeight: Math.min(root.panelHeight + headerHeight, root.hasMorphSource ? root.height - targetY - 10 : root.panelHeight)
        readonly property real targetX: root.alignRight ? root.width - root.panelWidth - 10 : root.hasMorphSource ? Math.max(10, Math.min(root.width - root.panelWidth - 10, root._source.bounds.x + (root._source.bounds.width - root.panelWidth) / 2)) : (root.panelTargetX >= 0 ? root.panelTargetX : (root.width - root.panelWidth) / 2)
        readonly property real targetY: root.hasMorphSource ? root._source.bounds.y : (root.panelTargetY >= 0 ? root.panelTargetY : (root.height - root.panelHeight) / 2)
        readonly property real sourceX: root.hasMorphSource ? root._returnBounds.x : targetX + root.closedOffsetX
        readonly property real sourceY: root.hasMorphSource ? root._returnBounds.y : targetY + root.closedOffsetY
        readonly property real sourceWidth: root.hasMorphSource ? root._returnBounds.width : root.panelWidth
        readonly property real sourceHeight: root.hasMorphSource ? root._returnBounds.height : root.panelHeight

        x: sourceX + (targetX - sourceX) * widthProgress
        y: sourceY + (targetY - sourceY) * progress
        width: sourceWidth + (root.panelWidth - sourceWidth) * widthProgress
        height: sourceHeight + (targetHeight - sourceHeight) * progress
        radius: root.hasMorphSource ? root._source.item.radius + (root.panelRadius - root._source.item.radius) * progress : root.panelRadius
        color: root.hasMorphSource ? Qt.tint(root._source.item.color, Colors.withAlpha(Colors.surface0, revealProgress * Tokens.panelAlpha)) : Colors.withAlpha(Colors.surface0, Tokens.panelAlpha)
        border.color: Colors.overlay(Tokens.borderAlpha)
        border.width: root.hasMorphSource ? root._source.item.border.width + (1 - root._source.item.border.width) * revealProgress : 1
        opacity: root.hasMorphSource ? (root._sourceHeld ? 1 : 0) : (root._atTarget ? 1 : 0)
        clip: true

        SmokedGlass {
            anchors.fill: parent
            radius: panel.radius
            opacity: panel.revealProgress
        }

        MouseArea {
            anchors.fill: parent
            onClicked: mouse => mouse.accepted = true
        }

        Item {
            id: headerDock
            width: parent.width
            height: panel.headerHeight
            visible: root.hasMorphSource

            MouseArea {
                anchors.fill: parent
                anchors.leftMargin: root.interactiveHeader ? parent.width - 36 : 0
                z: 1
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                cursorShape: Qt.PointingHandCursor
                onClicked: mouse => {
                    if (mouse.button === Qt.RightButton)
                        root._source.item.rightClicked(mouse);
                    else
                        root.closeRequested();
                }
            }

            Text {
                anchors.right: parent.right
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                text: "⌃"
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.title
                opacity: panel.revealProgress
                z: 2
            }
        }

        Rectangle {
            x: Tokens.spaceL
            y: panel.headerHeight
            width: parent.width - Tokens.spaceL * 2
            height: 1
            color: Colors.surface1
            opacity: root.hasMorphSource ? panel.revealProgress : 0
        }

        Flickable {
            id: contentViewport
            y: panel.headerHeight
            width: root.panelWidth
            height: Math.max(0, panel.height - y)
            contentWidth: width
            contentHeight: root.panelHeight
            interactive: root.showing && contentHeight > height
            clip: true
            opacity: root.hasMorphSource ? Math.min(1, panel.progress * 1.5) : (root._atTarget ? 1 : 0)
            enabled: root.showing

            Item {
                id: panelInner
                width: root.panelWidth
                height: root.panelHeight
            }
        }

        Behavior on opacity {
            enabled: !root.hasMorphSource
            NumberAnimation {
                duration: Tokens.animNormal
                easing.type: Easing.OutCubic
            }
        }
    }

    HoverHandler {
        id: cursorTracker
        parent: root.contentItem
        enabled: root.visible
        blocking: false
    }

    PanelBorderGlow {
        x: panel.x
        y: panel.y
        width: panel.width
        height: panel.height
        radius: panel.radius
        cursorPosition: Qt.point(cursorTracker.point.position.x - x, cursorTracker.point.position.y - y)
        opacity: panel.opacity * panel.revealProgress * (cursorTracker.hovered ? 1 : 0)
        visible: opacity > 0
    }
}
