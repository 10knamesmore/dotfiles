import "../theme"
import "../state"
import QtQuick
import Quickshell
import Quickshell.Hyprland
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
// SoftShadow 内部开了 layer 缓存走 GPU 合成；panel 容器本身含动态内容（ListView 等），
// 不给它开 layer——否则每帧都要重画整块 FBO，反而更贵。
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

    // ── 入场特效 ──
    //   Morph：有点击源（bar 模块点击）时从点击点的小块展开；无点击源时退回 Slide
    //   Slide：永远从 closedOffset 方向滑入（+ 淡入），忽略点击源
    // QML 限制：enum 属性只能声明为 int，引用必须写 PanelOverlay.Morph；
    // 裸写 Entrance.Morph 在 binding 里会静默判 false。
    enum Entrance { Morph, Slide }
    property int entrance: PanelOverlay.Morph

    // ── morph 源（打开时快照）──
    property real _morphX: -1
    property real _morphY: -1
    readonly property bool hasMorphSource: entrance === PanelOverlay.Morph && _morphX >= 0

    // ── 内容 ──
    default property alias panelContent: panelInner.data
    readonly property alias panel: panel

    // ── 动画状态 ──
    property bool _keepVisible: false
    property bool _atTarget: false    // true=目标位置，false=源位置
    property bool _animEnabled: false // 是否启用 Behavior 过渡

    signal closeRequested()

    // 新窗口在 compositor configure 之前 width/height 还是 Qt 默认值（500x500，panelHeight
    // 甚至会算出 0），此时开动画会让面板从左上角以错误尺寸“形变”展开。等窗口尺寸匹配到
    // 某块真实屏幕（说明 configure 完成）再启动展开动画。
    // 不能用 root.screen 比较：compositor 自选屏的 layer surface 上它可能是另一块屏
    // （实测窗口在 eDP-1 而 root.screen 报 DP-3）。
    readonly property bool _sizeReady: root.width > 0 && root.height > 0
        && _matchesAnyScreen(root.width, root.height)
    property bool _waitingForSize: false

    function _matchesAnyScreen(w, h) {
        for (let i = 0; i < Quickshell.screens.length; i++) {
            const s = Quickshell.screens[i];
            if (s.width === w && s.height === h)
                return true;
        }
        return false;
    }

    function _tryStartOpen() {
        if (!_waitingForSize || !_sizeReady)
            return;

        _waitingForSize = false;
        _openTimer.start();
    }

    onWidthChanged: _tryStartOpen()
    onHeightChanged: _tryStartOpen()

    onShowingChanged: {
        if (showing) {
            _morphX = MorphState.morphSourceX;
            _morphY = MorphState.morphSourceY;
            MorphState.reset(); // 点击源一次性：取完即清，不残留给下一次打开
            _animEnabled = false; // 关闭动画
            // 快速「关→开」时上一次关闭动画会被打断，Behavior 把面板冻结在中途值，
            // 此时仅把 _atTarget 设回 false 不会重写几何属性（值本来就已是 false）。
            // 先置 true 再置 false，让几何绑定在 Behavior 关闭状态下重写一次，把面板
            // 复位到源位置，下一 tick 的展开动画才能完整重播。
            _atTarget = true;
            _atTarget = false;
            _keepVisible = true;
            _hideTimer.stop();
            if (_sizeReady)
                _openTimer.start();   // 下一帧开始动画到目标
            else
                _waitingForSize = true; // 等 configure 给出真实窗口尺寸
        } else {
            _waitingForSize = false;
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

    // 面板与遮罩的淡出在 animNormal（250ms）内结束（morph 模式的面板淡出更早，230ms），
    // 之后的几何收缩已经不可见；早点 unmap，避免 surface 在不可见状态下继续映射与合成。
    Timer {
        id: _hideTimer
        interval: Tokens.animNormal + 50
        onTriggered: root._keepVisible = false
    }

    // Hyprland 只在 layer surface 重新 map 时授予 OnDemand 键盘焦点；快速重开会复用
    // 已映射的 surface，不会重新 map，必须显式 grab，否则按键穿透到下层窗口。
    HyprlandFocusGrab {
        windows: [root]
        active: root.showing
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
        color: Colors.withAlpha(Colors.base,
            root._atTarget ? Tokens.panelAlpha : (root.hasMorphSource ? Tokens.panelAlpha * 0.5 : Tokens.panelAlpha))
        border.color: Colors.overlay(root._atTarget ? Tokens.borderAlpha : 0)
        border.width: 1
        opacity: root.hasMorphSource ? _closeFade.fadeValue : (root._atTarget ? 1 : 0)
        clip: true


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
