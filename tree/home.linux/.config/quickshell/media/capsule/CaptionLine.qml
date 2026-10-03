import "../../theme"
import "../../state"
import "../../components"
import QtQuick

// 一帧标题或歌词。长文本往返滚动；有逐词时间时整词高亮，视口跟随演唱位置。
Item {
    id: root

    // AnimatedCaption 创建时传入的内容快照，换行退场期间不替换文字。
    required property var caption
    property color textColor: Colors.text
    readonly property bool wordTimed: caption.lyric && caption.words.length > 0
    readonly property var wordRanges: {
        let prefix = "";
        const ranges = [];
        for (const word of caption.words) {
            const left = textMetrics.advanceWidth(prefix);
            prefix += word.text;
            ranges.push({
                text: word.text,
                start: word.start,
                duration: word.duration,
                left: left,
                right: textMetrics.advanceWidth(prefix)
            });
        }
        return ranges;
    }
    // 连续演唱位置仅用于平滑移动视口，不再裁切词内的高亮区域。
    readonly property real playbackTextPosition: {
        const now = LyricsState.currentTimeMs;
        for (const word of wordRanges) {
            if (now < word.start)
                return word.left;
            if (word.duration > 0 && now < word.start + word.duration)
                return word.left + (word.right - word.left) * (now - word.start) / word.duration;
        }
        return wordTimed ? implicitWidth : 0;
    }
    readonly property real overflowWidth: Math.max(0, implicitWidth - width)
    readonly property real scrollOffset: wordTimed ? Math.max(0, Math.min(overflowWidth, playbackTextPosition - width * 0.6)) : autoScroll.offset

    implicitWidth: textMetrics.advanceWidth(caption.text)
    implicitHeight: 28
    clip: true
    Accessible.ignored: true

    FontMetrics {
        id: textMetrics
        font.family: Fonts.family
        font.pixelSize: Fonts.body
        font.weight: Font.Medium
    }

    TextScrollAnimation {
        id: autoScroll
        distance: root.overflowWidth
        running: !root.wordTimed && distance > 0
        // 字幕视口变化后按新距离重新计时；换行由 AnimatedCaption 创建新组件。
        onDistanceChanged: {
            if (running)
                restart();
        }
    }

    Item {
        x: -root.scrollOffset
        width: Math.max(root.width, root.implicitWidth)
        height: parent.height

        Text {
            anchors.verticalCenter: parent.verticalCenter
            visible: !root.wordTimed
            width: Math.max(root.width, root.implicitWidth)
            horizontalAlignment: Text.AlignHCenter
            text: root.caption.text
            textFormat: Text.PlainText
            font: textMetrics.font
            color: root.caption.lyric ? Colors.mauve : root.textColor
            elide: Text.ElideNone
            Behavior on color {
                ColorAnimation {
                    duration: Tokens.animFast
                }
            }
        }

        Repeater {
            model: root.wordTimed ? root.wordRanges : []

            delegate: Text {
                required property var modelData

                x: modelData.left
                anchors.verticalCenter: parent.verticalCenter
                text: modelData.text
                textFormat: Text.PlainText
                font: textMetrics.font
                color: {
                    const now = LyricsState.currentTimeMs;
                    if (now >= modelData.start + modelData.duration)
                        return Colors.text;
                    if (now >= modelData.start)
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
}
