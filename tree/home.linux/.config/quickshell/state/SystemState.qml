pragma Singleton
import QtQuick

// 系统状态 — 通知计数 / 清空通知信号
QtObject {
    property int notificationCount: 0

    signal clearAllNotifications
}
