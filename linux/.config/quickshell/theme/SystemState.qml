import QtQuick
pragma Singleton

// 系统状态 — 勿扰开关 / 通知计数 / 清空通知信号
QtObject {
    property bool dndEnabled: false
    property int notificationCount: 0

    signal clearAllNotifications()
}
