import "../state"
import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// 歌词区右上角的显示设置，直接修改共享的翻译和时间偏移状态。
Popup {
    id: root

    function adjustOffset(delta) {
        LyricsState.lyricsOffset = Math.round((LyricsState.lyricsOffset + delta) * 10) / 10;
        console.debug("[media-panel] lyrics offset:", LyricsState.lyricsOffset);
    }

    width: 224
    padding: 16
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    background: Rectangle {
        radius: Tokens.radiusM
        color: Colors.mantle
        border.color: Colors.overlay(0.12)
    }

    contentItem: ColumnLayout {
        spacing: 10

        Text {
            text: "歌词设置"
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.small
        }

        LanguageToggle {
            text: "中文翻译"
            visible: LyricsState.hasTranslation
            checked: LyricsState.showTranslation
            onToggled: LyricsState.showTranslation = checked
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 2

            Text {
                Layout.fillWidth: true
                text: "时间偏移"
                color: Colors.subtext0
                font.family: Fonts.family
                font.pixelSize: Fonts.caption
            }

            MediaButton {
                implicitWidth: 26
                implicitHeight: 28
                text: "−"
                label: "歌词延后 0.1 秒"
                onClicked: root.adjustOffset(-0.1)
            }

            Button {
                id: resetOffset

                implicitWidth: 52
                implicitHeight: 28
                hoverEnabled: true
                Accessible.name: "重置歌词偏移"
                onClicked: {
                    LyricsState.lyricsOffset = 0;
                    console.debug("[media-panel] lyrics offset reset");
                }

                background: Rectangle {
                    radius: Tokens.radiusS
                    color: resetOffset.hovered || resetOffset.visualFocus ? Colors.overlay(0.07) : "transparent"
                }

                contentItem: Text {
                    text: (LyricsState.lyricsOffset >= 0 ? "+" : "") + LyricsState.lyricsOffset.toFixed(1) + "s"
                    color: LyricsState.lyricsOffset === 0 ? Colors.subtext0 : Colors.mauve
                    font.family: Fonts.family
                    font.pixelSize: Fonts.caption
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }

            }

            MediaButton {
                implicitWidth: 26
                implicitHeight: 28
                text: "+"
                label: "歌词提前 0.1 秒"
                onClicked: root.adjustOffset(0.1)
            }

        }

        Text {
            text: "正值提前，负值延后；点击数值重置。"
            color: Colors.overlay2
            font.family: Fonts.family
            font.pixelSize: Fonts.xs
        }

    }

    component LanguageToggle: CheckBox {
        id: toggle

        Layout.fillWidth: true
        implicitHeight: 28
        leftPadding: 0
        rightPadding: 0
        hoverEnabled: true

        contentItem: Text {
            text: toggle.text
            color: Colors.subtext0
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
            verticalAlignment: Text.AlignVCenter
            rightPadding: 36
        }

        indicator: Rectangle {
            x: toggle.width - width
            y: (toggle.height - height) / 2
            implicitWidth: 28
            implicitHeight: 16
            radius: 8
            color: toggle.checked ? Colors.withAlpha(Colors.mauve, 0.4) : Colors.surface1
            border.width: toggle.visualFocus ? 1 : 0
            border.color: Colors.mauve

            Rectangle {
                x: toggle.checked ? 14 : 2
                y: 2
                width: 12
                height: 12
                radius: 6
                color: toggle.checked ? Colors.mauve : Colors.subtext0

                Behavior on x {
                    NumberAnimation {
                        duration: Tokens.animFast
                        easing.type: Easing.OutCubic
                    }

                }

            }

        }

    }

    enter: Transition {
        NumberAnimation {
            property: "opacity"
            from: 0
            to: 1
            duration: 150
        }

    }

    exit: Transition {
        NumberAnimation {
            property: "opacity"
            to: 0
            duration: 100
        }

    }

}
