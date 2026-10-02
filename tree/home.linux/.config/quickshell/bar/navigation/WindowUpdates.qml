pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Hyprland._Ipc

// 所有显示器共享原生窗口模型；几何与焦点历史走「首个事件立即刷 + 30ms 合并尾刷」。
// 标题由原生模型实时更新，不为标题变化额外请求窗口列表。
Scope {
    readonly property var toplevels: Hyprland.toplevels

    Component.onCompleted: Hyprland.refreshToplevels()

    Connections {
        target: Hyprland
        function onRawEvent(event) {
            if (event.name === "activespecial" || event.name === "activespecialv2")
                Hyprland.refreshMonitors();
            if (event.name === "windowtitle" || event.name === "windowtitlev2" || event.name === "activelayout" || event.name === "submap")
                return;
            requestRefresh();
        }
    }

    // 第一个事件立即刷新，move 这类单发事件不用等合并窗口；
    // 随后 30ms 内的同类事件合并成一次尾刷。
    // refreshToplevels 在途时会丢弃调用，尾刷同时兜住这种情况。
    function requestRefresh() {
        if (!refreshTimer.running)
            Hyprland.refreshToplevels();
        refreshTimer.restart();
    }

    Timer {
        id: refreshTimer
        interval: 30
        onTriggered: Hyprland.refreshToplevels()
    }
}
