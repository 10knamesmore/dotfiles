import QtQuick
pragma Singleton

// 桌面浮动组件可见性开关
QtObject {
    property bool analogClockVisible: true
    property bool pomodoroVisible: true
    property bool visualizerVisible: true
    property bool weatherWidgetVisible: true
    property bool nowPlayingVisible: true
    property bool systemMonitorVisible: true
}
