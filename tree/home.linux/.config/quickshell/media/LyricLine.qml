import "../theme"
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// 一行歌词及翻译。所有播放状态使用相同字号和换行，按时间将正在唱的整个词高亮。
Button {
    id: root

    required property var line
    required property bool current
    required property real playbackTimeMs
    required property bool showTranslation
    property bool browsing: false
    property int distance: 0
    readonly property bool wordTimed: line.words.length > 0

    implicitHeight: Math.max(76, lineContent.implicitHeight + topPadding + bottomPadding)
    leftPadding: 6
    rightPadding: 6
    topPadding: 12
    bottomPadding: 12
    hoverEnabled: true
    opacity: current || hovered || visualFocus ? 1 : browsing ? 0.75 : distance < 2 ? 0.65 : 0.4
    Accessible.name: line.text
    Accessible.description: "跳转到这句歌词"

    background: Rectangle {
        radius: Tokens.radiusS
        color: root.hovered || root.visualFocus ? Colors.overlay(0.035) : "transparent"
    }

    contentItem: ColumnLayout {
        id: lineContent

        spacing: 7

        Flow {
            Layout.fillWidth: true
            visible: root.wordTimed
            spacing: 0

            Repeater {
                model: root.wordTimed ? root.line.words : []

                delegate: Text {
                    required property var modelData

                    text: modelData.text
                    textFormat: Text.PlainText
                    font.family: Fonts.family
                    font.pixelSize: Fonts.heading
                    font.weight: Font.Medium
                    color: {
                        if (!root.current)
                            return Colors.subtext0;

                        if (root.playbackTimeMs >= modelData.start + modelData.duration)
                            return Colors.text;

                        if (root.playbackTimeMs >= modelData.start)
                            return Colors.mauve;

                        return Colors.overlay1;
                    }
                    Accessible.ignored: true

                    Behavior on color {
                        ColorAnimation {
                            duration: Tokens.animFast
                        }

                    }

                }

            }

        }

        Text {
            Layout.fillWidth: true
            visible: !root.wordTimed
            text: root.line.text
            textFormat: Text.PlainText
            font.family: Fonts.family
            font.pixelSize: Fonts.heading
            font.weight: Font.Medium
            color: root.current ? Colors.mauve : Colors.subtext0
            wrapMode: Text.Wrap
            Accessible.ignored: true
        }

        Text {
            Layout.fillWidth: true
            visible: root.showTranslation && text.length > 0
            text: root.line.translation
            textFormat: Text.PlainText
            font.family: Fonts.family
            font.pixelSize: Fonts.caption
            color: Colors.subtext0
            wrapMode: Text.Wrap
        }

    }

    Behavior on opacity {
        NumberAnimation {
            duration: 280
            easing.type: Easing.OutCubic
        }

    }

}
