import "../theme"
import "../state"
import QtQuick
import Quickshell
import Quickshell.Wayland

// 通用面板 overlay — morph / 滑入 / 淡入动画
//
// 视觉设计：
//   - 有 morph 源（bar 模块点击等）：从点击点的 40×40 小块「展开」到目标位置目标尺寸，
//     子元素跟着 anchors 重排显现，给人「光柱展开」感
//   - 无 morph 源（全局快捷键唤起）：从 closedOffset 方向 slide 到目标 + 淡入
//
// 性能权衡：morph 路径会触发子元素 relayout（这是「展开」感的根源，不可避免），
// 但只发生在 ~500ms 动画期内；静态后无消耗。
// SoftShadow / panel container 都开了 layer 缓存，让 hover 等微动画走 GPU 合成。
PanelWindow {
    id: root

    // ── 必须绑定 ──
    property bool showing: false

    // ── 遮罩 ──
    property real backdropOpacity: Tokens.backdropDim

    // ── 面板目标属性 ──
    property real panelWidth: 400
    property real panelHeight: 400
    property int panelRadius: Tokens.radiusL

    // ── 目标位置（-1 = 自动居中）──
    property real panelTargetX: -1
    property real panelTargetY: -1

    // ── 关闭态偏移（无 morph 源时的关闭位移）──
    property real closedOffsetX: 0
    property real closedOffsetY: 20

    // ── morph 源（打开时快照）──
    property real _morphX: -1
    property real _morphY: -1
    readonly property bool hasMorphSource: _morphX >= 0

    // ── 内容 ──
    default property alias panelContent: panelInner.data
    readonly property alias panel: panel

    // ── 动画状态 ──
    property bool _keepVisible: false
    property bool _atTarget: false    // true=目标位置，false=源位置
    property bool _animEnabled: false // 是否启用 Behavior 过渡

    signal closeRequested()

    onShowingChanged: {
        if (showing) {
            _morphX = MorphState.morphSourceX;
            _morphY = MorphState.morphSourceY;
            _animEnabled = false; // 关闭动画
            _atTarget = false;    // 跳到源位置（无动画）
            _keepVisible = true;
            _hideTimer.stop();
            _openTimer.start();   // 下一帧开始动画到目标
        } else {
            _atTarget = false;    // 动画回到源位置
            _hideTimer.start();
        }
    }

    Timer {
        id: _openTimer
        interval: 0
        onTriggered: {
            root._animEnabled = true;
            root._atTarget = true;
        }
    }

    Timer {
        id: _hideTimer
        interval: Tokens.animElaborate + 50
        onTriggered: root._keepVisible = false
    }

    // 关闭时延迟淡出（morph 模式）— 80ms 停顿后 150ms 淡出，
    // 让用户先看到面板"开始收缩"再消失
    Item {
        id: _closeFade
        property real fadeValue: 1

        states: [
            State {
                name: "open"; when: root.showing
                PropertyChanges { target: _closeFade; fadeValue: 1 }
            },
            State {
                name: "closed"; when: !root.showing
                PropertyChanges { target: _closeFade; fadeValue: 0 }
            }
        ]

        transitions: [
            Transition {
                from: "open"; to: "closed"
                SequentialAnimation {
                    PauseAnimation { duration: 80 }
                    NumberAnimation {
                        target: _closeFade; property: "fadeValue"
                        duration: 150; easing.type: Easing.OutQuint
                    }
                }
            },
            Transition {
                from: "closed"; to: "open"
                NumberAnimation {
                    target: _closeFade; property: "fadeValue"
                    duration: 0
                }
            }
        ]
    }

    anchors.top: true
    anchors.bottom: true
    anchors.left: true
    anchors.right: true
    visible: showing || _keepVisible
    focusable: showing
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"

    // ── 遮罩 ──
    Rectangle {
        anchors.fill: parent
        color: "#000000"
        opacity: root.showing ? root.backdropOpacity : 0

        Behavior on opacity {
            NumberAnimation {
                duration: Tokens.animNormal
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Anim.standard
            }
        }
    }

    // Esc 关闭
    Item {
        focus: root.showing
        Keys.onEscapePressed: root.closeRequested()
    }

    // 点击外部关闭
    MouseArea {
        anchors.fill: parent
        onClicked: root.closeRequested()
    }

    // ── panel 容器（几何动画 morph）──
    Rectangle {
        id: panel

        property real targetX: root.panelTargetX >= 0 ? root.panelTargetX : (root.width - root.panelWidth) / 2
        property real targetY: root.panelTargetY >= 0 ? root.panelTargetY : (root.height - root.panelHeight) / 2
        property real srcX: root.hasMorphSource ? root._morphX - 20 : targetX + root.closedOffsetX
        property real srcY: root.hasMorphSource ? root._morphY - 20 : targetY + root.closedOffsetY
        property real srcW: root.hasMorphSource ? 40 : root.panelWidth
        property real srcH: root.hasMorphSource ? 40 : root.panelHeight
        property int srcR: root.hasMorphSource ? 20 : root.panelRadius

        x: root._atTarget ? targetX : srcX
        y: root._atTarget ? targetY : srcY
        width: root._atTarget ? root.panelWidth : srcW
        height: root._atTarget ? root.panelHeight : srcH
        radius: root._atTarget ? root.panelRadius : srcR
        color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b,
            root._atTarget ? Tokens.panelAlpha : (root.hasMorphSource ? Tokens.panelAlpha * 0.5 : Tokens.panelAlpha))
        border.color: Qt.rgba(1, 1, 1, root._atTarget ? Tokens.borderAlpha : 0)
        border.width: 1
        opacity: root.hasMorphSource ? _closeFade.fadeValue : (root._atTarget ? 1 : 0)
        clip: true

        SoftShadow {
            anchors.fill: parent
            radius: parent.radius
        }

        MouseArea {
            anchors.fill: parent
            onClicked: mouse => mouse.accepted = true
        }

        Item {
            id: panelInner
            anchors.fill: parent
            opacity: root._atTarget ? 1 : 0

            Behavior on opacity {
                enabled: root._animEnabled
                NumberAnimation {
                    duration: root.hasMorphSource ? Tokens.animNormal : Tokens.animFast
                    easing.type: Easing.OutCubic
                }
            }
        }

        InnerGlow {}

        Behavior on x {
            enabled: root._animEnabled
            NumberAnimation { duration: Tokens.animElaborate; easing.type: Easing.OutQuint }
        }
        Behavior on y {
            enabled: root._animEnabled
            NumberAnimation { duration: Tokens.animElaborate; easing.type: Easing.OutQuint }
        }
        Behavior on width {
            enabled: root._animEnabled
            NumberAnimation { duration: Tokens.animElaborate; easing.type: Easing.OutQuint }
        }
        Behavior on height {
            enabled: root._animEnabled
            NumberAnimation { duration: Tokens.animElaborate; easing.type: Easing.OutQuint }
        }
        Behavior on radius {
            enabled: root._animEnabled
            NumberAnimation { duration: Tokens.animElaborate; easing.type: Easing.OutQuint }
        }
        Behavior on opacity {
            enabled: root._animEnabled
            NumberAnimation {
                duration: Tokens.animNormal
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Anim.standard
            }
        }
        Behavior on color {
            enabled: root._animEnabled
            ColorAnimation { duration: Tokens.animSlow }
        }
        Behavior on border.color {
            enabled: root._animEnabled
            ColorAnimation { duration: Tokens.animSlow }
        }
    }
}
