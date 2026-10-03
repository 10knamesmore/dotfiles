import "../../theme"
import "../../state"
import "../components"
import QtQuick
import Quickshell

// 悬停显示时间与日期；月历延续同一行内容，收起时日期与轮廓同步回到紧凑态。
BarModule {
    id: root

    property bool showDate: false
    readonly property real compactWidth: timeText.implicitWidth + (showDate ? 0 : secondsText.implicitWidth) + horizontalPadding * 2
    readonly property real hoverWidth: timeText.implicitWidth + (showDate ? 0 : dateText.implicitWidth) + horizontalPadding * 2

    function openCalendar() {
        PanelState.closeAll();
        MorphState.openFrom(root, () => PanelState.toggleCalendar());
    }

    accentColor: Colors.blue
    implicitWidth: hovered ? hoverWidth : compactWidth
    activeFocusOnTab: true
    Accessible.role: Accessible.Button
    Accessible.name: "日历"
    Accessible.description: Qt.formatDateTime(clock.date, "yyyy/MM/dd ddd HH:mm:ss")
    Accessible.checkable: true
    Accessible.checked: PanelState.calendarOpen
    Accessible.onPressAction: root.openCalendar()
    Keys.onReturnPressed: root.openCalendar()
    Keys.onSpacePressed: root.openCalendar()
    onClicked: root.openCalendar()
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
            text: Qt.formatTime(clock.date, ":ss")
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
