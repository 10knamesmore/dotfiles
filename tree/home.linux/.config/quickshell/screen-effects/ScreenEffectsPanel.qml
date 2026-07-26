import "../components"
import "../theme"
import "../state"
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

// 屏幕效果控制面板 — 作为 layer-shell overlay 显示在右上角。
//
// 纯 UI：状态一律读 ScreenEffectsState，操作一律发意图信号，落地逻辑（状态持久化、
// GLSL 生成、hyprctl 热加载、背光调节）全在 services/ScreenEffectsService.qml。
PanelOverlay {
    id: root

    function togglePanel() {
        PanelState.calendarOpen = false;
        PanelState.mediaOpen = false;
        PanelState.toggleScreenEffects();
    }

    showing: PanelState.screenEffectsOpen
    panelWidth: 320
    panelHeight: col.implicitHeight + 32
    panelTargetX: root.width - 330
    panelTargetY: 54
    closedOffsetY: -20
    onCloseRequested: PanelState.screenEffectsOpen = false
    onShowingChanged: {
        if (showing)
            ScreenEffectsState.refresh(); // 回读背光实际值（可能被亮度键改过）
    }

    ColumnLayout {
        id: col

        anchors.fill: parent
        anchors.margins: Tokens.spaceL
        spacing: 6

        // 标题
        RowLayout {
            Layout.fillWidth: true

            Text {
                text: "屏幕效果"
                font.family: Fonts.family
                font.pixelSize: Fonts.title
                font.bold: true
                color: Colors.text
            }

            Item {
                Layout.fillWidth: true
            }

            ToggleSwitch {
                checked: ScreenEffectsState.effectsActive
                onToggled: ScreenEffectsState.toggle()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: Colors.surface1
        }

        // ── 滑块 ──
        EffectSlider {
            label: "☀ 亮度"
            value: ScreenEffectsState.brightness
            onMoved: val => ScreenEffectsState.setBrightness(val)
        }

        EffectSlider {
            label: "🌙 色温"
            value: ScreenEffectsState.warmth
            onMoved: val => ScreenEffectsState.setWarmth(val)
        }

        EffectSlider {
            label: "🎞 颗粒强度"
            value: ScreenEffectsState.grain
            onMoved: val => ScreenEffectsState.setGrain(val)
        }

        EffectSlider {
            label: "◐ 颗粒大小"
            value: ScreenEffectsState.grainSize
            onMoved: val => ScreenEffectsState.setGrainSize(val)
        }

        EffectSlider {
            label: "◑ 暗部增强"
            value: ScreenEffectsState.shadowBoost
            onMoved: val => ScreenEffectsState.setShadowBoost(val)
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: Colors.surface1
        }

        // ── 预设（warmth, grain, grainSize, shadowBoost）──
        // warmth 的档位对应色温：0=6500K 中性、35≈5100K、60=4100K。
        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            PresetButton {
                text: "关闭"
                onClicked: ScreenEffectsState.requestApply(0, 0, 50, 40)
            }

            PresetButton {
                text: "阅读"
                onClicked: ScreenEffectsState.requestApply(60, 85, 10, 40)
            }

            PresetButton {
                text: "Portra"
                onClicked: ScreenEffectsState.requestApply(35, 35, 45, 50)
            }

            PresetButton {
                text: "Tri-X"
                onClicked: ScreenEffectsState.requestApply(0, 60, 55, 70)
            }
        }
    }

}
