import "../state"
import QtQuick
import Quickshell
import Quickshell.Services.Notifications

// 通知服务：DBus 通知服务端，向 SystemState 同步计数并响应清空请求。
// server 供控制中心的 NotificationPage 与 NotificationToast 渲染。
Scope {
    id: root

    readonly property alias server: notifServer

    NotificationServer {
        id: notifServer

        keepOnReload: true
        bodySupported: true
        bodyMarkupSupported: true
        imageSupported: true
        actionsSupported: true
        persistenceSupported: true
        onNotification: notification => {
            notification.tracked = true;
        }
    }

    Connections {
        function onObjectInsertedPost() {
            SystemState.notificationCount = notifServer.trackedNotifications.values.length;
        }

        function onObjectRemovedPost() {
            SystemState.notificationCount = notifServer.trackedNotifications.values.length;
        }

        target: notifServer.trackedNotifications
    }

    Connections {
        function onClearAllNotifications() {
            let vals = notifServer.trackedNotifications.values;
            for (let i = vals.length - 1; i >= 0; i--) {
                vals[i].dismiss();
            }
        }

        target: SystemState
    }
}
