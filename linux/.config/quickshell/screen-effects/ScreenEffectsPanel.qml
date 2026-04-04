import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

// 屏幕效果控制面板 — 作为 layer-shell overlay 显示在右上角
PanelWindow {
    // ── UI ──

    id: root

    // 双阶段可见性
    property bool showing: PanelState.screenEffectsOpen
    property bool animating: _opacityAnim.running || _slideAnim.running
    // ── 状态管理 ──
    property string home: Quickshell.env("HOME")
    property string stateFilePath: home + "/.cache/hypr/screen-effects.json"
    property string scriptPath: home + "/dotfiles/generated/scripts/hypr/screen_effects.sh"
    property int warmth: 0
    property int grain: 0
    property int grainSize: 50
    property int shadowBoost: 40
    property int brightness: 100
    property bool effectsActive: warmth > 0 || grain > 0

    function togglePanel() {
        PanelState.calendarOpen = false;
        PanelState.mediaOpen = false;
        PanelState.toggleScreenEffects();
    }

    function loadState() {
        stateReader.command = ["cat", stateFilePath];
        stateReader.running = true;
    }

    function readBrightness() {
        brightnessReader.command = ["brightnessctl", "-m"];
        brightnessReader.running = true;
    }

    function applyPreset(w, g, gs, sb) {
        warmth = w;
        grain = g;
        grainSize = gs;
        shadowBoost = sb;
        saveAndApply();
    }

    function saveAndApply() {
        let json = JSON.stringify({
            "warmth": warmth,
            "grain": grain,
            "grain_size": grainSize,
            "shadow_boost": shadowBoost
        });
        writer.command = ["sh", "-c", "echo '" + json + "' > " + stateFilePath];
        writer.running = true;
        applier.command = [scriptPath, "apply"];
        applier.running = true;
    }

    // 铺满全屏，背景透明，点击面板外部关闭
    anchors.top: true
    anchors.bottom: true
    anchors.left: true
    anchors.right: true
    visible: showing || animating
    focusable: false
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"
    onShowingChanged: {
        if (showing) {
            loadState();
            readBrightness();
        }
    }

    // ── 进程 ──
    Process {
        id: stateReader

        stdout: SplitParser {
            onRead: (data) => {
                try {
                    let obj = JSON.parse(data);
                    root.warmth = obj.warmth ?? 0;
                    root.grain = obj.grain ?? 0;
                    root.grainSize = obj.grain_size ?? 50;
                    root.shadowBoost = obj.shadow_boost ?? 40;
                } catch (e) {
                }
            }
        }

    }

    Process {
        id: brightnessReader

        stdout: SplitParser {
            onRead: (data) => {
                let parts = data.split(",");
                if (parts.length >= 4)
                    root.brightness = parseInt(parts[3]) || 100;

            }
        }

    }

    Process {
        id: writer
    }

    Process {
        id: applier
    }

    Process {
        id: brightnessSetter
    }

    // 半透明遮罩
    Rectangle {
        anchors.fill: parent
        color: "#000000"
        opacity: root.showing ? 0.15 : 0

        Behavior on opacity {
            NumberAnimation {
                duration: 250
                easing.type: Easing.OutCubic
            }

        }

    }

    // 点击面板外部关闭
    MouseArea {
        anchors.fill: parent
        onClicked: PanelState.screenEffectsOpen = false
    }

    Rectangle {
        id: panel

        width: 320
        height: col.implicitHeight + 32
        radius: 16
        color: Qt.rgba(Colors.base.r, Colors.base.g, Colors.base.b, Tokens.panelAlpha)
        border.color: Qt.rgba(1, 1, 1, Tokens.borderAlpha)
        border.width: 1
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.topMargin: root.showing ? 54 : 34
        anchors.rightMargin: 10
        opacity: root.showing ? 1 : 0

        // 阻止点击面板内部时触发背景 MouseArea
        MouseArea {
            anchors.fill: parent
            onClicked: (mouse) => {
                return mouse.accepted = true;
            }
        }

        ColumnLayout {
            id: col

            anchors.fill: parent
            anchors.margins: 16
            spacing: 6

            // 标题
            RowLayout {
                Layout.fillWidth: true

                Text {
                    text: "屏幕效果"
                    font.family: Fonts.family
                    font.pixelSize: Fonts.title
                    font.bold: true
                    color: "#cad3f5"
                }

                Item {
                    Layout.fillWidth: true
                }

                Switch {
                    id: toggleSwitch

                    checked: root.effectsActive
                    onToggled: {
                        if (!checked) {
                            root.warmth = 0;
                            root.grain = 0;
                            root.saveAndApply();
                        }
                    }

                    indicator: Rectangle {
                        implicitWidth: 40
                        implicitHeight: 20
                        radius: 10
                        color: toggleSwitch.checked ? "#a6da95" : "#5b6078"

                        Rectangle {
                            x: toggleSwitch.checked ? parent.width - width - 2 : 2
                            y: 2
                            width: 16
                            height: 16
                            radius: 8
                            color: "#cad3f5"

                            Behavior on x {
                                NumberAnimation {
                                    duration: 150
                                }

                            }

                        }

                    }

                }

            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: "#494d64"
            }

            // ── 滑块 ──
            EffectSlider {
                label: "☀ 亮度"
                value: root.brightness
                onMoved: (val) => {
                    root.brightness = val;
                    brightnessSetter.command = [root.scriptPath, "brightness", String(val)];
                    brightnessSetter.running = true;
                }
            }

            EffectSlider {
                label: "🌙 色温"
                value: root.warmth
                onMoved: (val) => {
                    root.warmth = val;
                    root.saveAndApply();
                }
            }

            EffectSlider {
                label: "🎞 颗粒强度"
                value: root.grain
                onMoved: (val) => {
                    root.grain = val;
                    root.saveAndApply();
                }
            }

            EffectSlider {
                label: "◐ 颗粒大小"
                value: root.grainSize
                onMoved: (val) => {
                    root.grainSize = val;
                    root.saveAndApply();
                }
            }

            EffectSlider {
                label: "◑ 暗部增强"
                value: root.shadowBoost
                onMoved: (val) => {
                    root.shadowBoost = val;
                    root.saveAndApply();
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: "#494d64"
            }

            // ── 预设 ──
            RowLayout {
                Layout.fillWidth: true
                spacing: 6

                PresetButton {
                    text: "关闭"
                    onClicked: root.applyPreset(0, 0, 50, 40)
                }

                PresetButton {
                    text: "阅读"
                    onClicked: root.applyPreset(60, 85, 10, 40)
                }

                PresetButton {
                    text: "Portra"
                    onClicked: root.applyPreset(20, 35, 45, 50)
                }

                PresetButton {
                    text: "Tri-X"
                    onClicked: root.applyPreset(0, 60, 55, 70)
                }

            }

        }

        // 内发光（毛玻璃顶部光源）
        Rectangle {
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            height: 1
            z: 1

            gradient: Gradient {
                orientation: Gradient.Horizontal

                GradientStop {
                    position: 0
                    color: "transparent"
                }

                GradientStop {
                    position: 0.3
                    color: Qt.rgba(1, 1, 1, 0.06)
                }

                GradientStop {
                    position: 0.7
                    color: Qt.rgba(1, 1, 1, 0.06)
                }

                GradientStop {
                    position: 1
                    color: "transparent"
                }

            }

        }

        Behavior on anchors.topMargin {
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

                duration: 300
                easing.type: Easing.OutCubic
            }

        }

    }

}
