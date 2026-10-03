import "../components"
import "../theme"
import "../state"
import QtQuick
import QtQuick.Layouts

// 控制中心详情页中的完整屏幕效果调节；状态与应用操作由 ScreenEffectsState / Service 承担。
Item {
    id: root

    required property bool showing
    implicitHeight: col.implicitHeight + Tokens.spaceL * 2
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
                Accessible.role: Accessible.CheckBox
                Accessible.name: "屏幕效果"
                Accessible.checkable: true
                Accessible.checked: checked
                Accessible.onToggleAction: ScreenEffectsState.toggle()
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
    }
}
