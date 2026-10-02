pragma ComponentBehavior: Bound

import "../../theme"
import QtQuick
import QtQuick.Controls

// 标题与歌词共用进出场动画。StackView 管理旧行退场及销毁；快速跳转只展示最新目标行。
StackView {
    id: root

    // key 区分曲目、标题和歌词行；重复歌词也能按行号触发动画。
    required property var caption
    property color textColor: Colors.text

    implicitHeight: 28
    clip: true
    background: null
    // 这里只负责显示；悬停交给覆盖整个胶囊的 BarModule.MouseArea。
    hoverEnabled: false

    function updateCaption() {
        if (busy)
            return;
        const line = currentItem as CaptionLine;
        if (empty) {
            pushItem(lineComponent, {
                caption: root.caption
            }, StackView.Immediate);
        } else if (line.caption.key !== caption.key) {
            replaceCurrentItem(lineComponent, {
                caption: root.caption
            }, StackView.ReplaceTransition);
        } else {
            line.caption = caption;
        }
    }

    onCaptionChanged: Qt.callLater(updateCaption)
    onBusyChanged: if (!busy)
        Qt.callLater(updateCaption)
    Component.onCompleted: updateCaption()

    Component {
        id: lineComponent
        CaptionLine {
            textColor: root.textColor
        }
    }

    replaceEnter: Transition {
        NumberAnimation {
            property: "y"
            from: 12
            to: 0
            duration: 260
            easing.type: Easing.OutCubic
        }
        NumberAnimation {
            property: "opacity"
            from: 0
            to: 1
            duration: 220
            easing.type: Easing.OutCubic
        }
    }
    replaceExit: Transition {
        NumberAnimation {
            property: "y"
            from: 0
            to: -12
            duration: 220
            easing.type: Easing.OutCubic
        }
        NumberAnimation {
            property: "opacity"
            from: 1
            to: 0
            duration: 180
            easing.type: Easing.OutCubic
        }
    }
}
