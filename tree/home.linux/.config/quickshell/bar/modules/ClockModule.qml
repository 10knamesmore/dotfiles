import "../../theme"
import "../../state"
import "../components"
import QtQuick
import Quickshell

// 悬停显示时间与日期；月历延续同一行内容，收起时日期与轮廓同步回到紧凑态。
BarModule {
    id: root

    property bool showDate: false
    readonly property real compactWidth: timeText.implicitWidth + (showDate ? 0 : secondsText.implicitWidth) + 36
    readonly property real hoverWidth: timeText.implicitWidth + (showDate ? 0 : dateText.implicitWidth) + 36

    radius: 20
    accentColor: Colors.blue
    implicitWidth: hovered ? hoverWidth : compactWidth
    onClicked: {
        PanelState.closeAll();
        MorphState.openFrom(root, () => PanelState.toggleCalendar());
    }
    onRightClicked: root.showDate = !root.showDate

    SystemClock {
        id: clock
        precision: SystemClock.Seconds
    }

    Row {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        spacing: 0

        Text {
            id: timeText
            text: root.showDate ? Qt.formatDate(clock.date, "dddd, MMMM d, yyyy") + " 󰃰" : Qt.formatTime(clock.date, "HH:mm")
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.ExtraBold
        }
        Text {
            id: secondsText
            text: Qt.formatTime(clock.date, ":ss") + " "
            visible: !root.showDate
            width: implicitWidth * (1 - root.detailProgress)
            opacity: 1 - root.detailProgress
            clip: true
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.ExtraBold
        }
        Text {
            id: dateText
            text: "  " + Qt.formatDate(clock.date, "MM/dd ddd")
            visible: !root.showDate
            width: implicitWidth * root.detailProgress
            opacity: root.detailProgress
            clip: true
            color: Colors.text
            font.family: Fonts.family
            font.pixelSize: Fonts.bodyLarge
            font.weight: Font.ExtraBold
        }
    }
}
