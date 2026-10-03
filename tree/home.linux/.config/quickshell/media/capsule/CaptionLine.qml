import "../../theme"
import "../../state"
import QtQuick

// 一帧标题或歌词。长文本往返滚动；有逐字时间的歌词跟随演唱位置。
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
                start: word.start,
                duration: word.duration,
                left: left,
                right: textMetrics.advanceWidth(prefix)
            });
        }
        return ranges;
    }
    readonly property real sungWidth: {
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
    property real autoScrollProgress: 0
    // 自动滚动速度为 30 px/s；首尾停留，短距离至少用一秒完成。
    readonly property int autoScrollDuration: Math.max(1000, Math.round(overflowWidth / 30 * 1000))
    readonly property real scrollOffset: wordTimed ? Math.max(0, Math.min(overflowWidth, sungWidth - width * 0.6)) : overflowWidth * autoScrollProgress

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

    SequentialAnimation {
        running: !root.wordTimed && root.overflowWidth > 0
        loops: Animation.Infinite
        onStopped: root.autoScrollProgress = 0

        PauseAnimation { duration: 1200 }
        NumberAnimation {
            target: root
            property: "autoScrollProgress"
            from: 0
            to: 1
            duration: root.autoScrollDuration
        }
        PauseAnimation { duration: 1200 }
        NumberAnimation {
            target: root
            property: "autoScrollProgress"
            from: 1
            to: 0
            duration: root.autoScrollDuration
        }
    }

    Item {
        x: -root.scrollOffset
        width: Math.max(root.width, root.implicitWidth)
        height: parent.height

        Text {
            anchors.verticalCenter: parent.verticalCenter
            width: root.wordTimed ? root.implicitWidth : Math.max(root.width, root.implicitWidth)
            horizontalAlignment: root.wordTimed ? Text.AlignLeft : Text.AlignHCenter
            text: root.caption.text
            textFormat: Text.PlainText
            font: textMetrics.font
            color: root.wordTimed ? Colors.overlay1 : (root.caption.lyric ? Colors.mauve : root.textColor)
            elide: Text.ElideNone
            Behavior on color {
                ColorAnimation {
                    duration: Tokens.animFast
                }
            }
        }

        Item {
            width: root.sungWidth
            height: parent.height
            visible: root.wordTimed
            clip: true

            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: root.caption.text
                textFormat: Text.PlainText
                font: textMetrics.font
                color: Colors.mauve
            }
        }
    }
}
