import QtQuick
import Quickshell.Hyprland._Ipc

// 按显示器选择可见工作区、最近聚焦窗口和缩略图位置，标题与布局共用此上下文。
QtObject {
    id: root

    required property var barScreen
    readonly property var monitor: barScreen ? Hyprland.monitorFor(barScreen) : null
    readonly property int workspaceId: monitor ? (monitor.lastIpcObject.specialWorkspace?.id || monitor.activeWorkspace?.id || 0) : 0
    readonly property bool focused: monitor ? monitor.focused : false
    readonly property var windows: WindowUpdates.toplevels.values.filter(window => window.workspace && window.workspace.id === root.workspaceId && window.lastIpcObject.mapped)

    // 非焦点屏仍显示该屏最近操作的窗口，不跟随另一屏的全局焦点。
    readonly property var activeWindow: {
        const active = Hyprland.activeToplevel;
        if (active && windows.includes(active))
            return active;
        let recent = null;
        for (const window of windows) {
            const order = window.lastIpcObject.focusHistoryID;
            if (order >= 0 && (!recent || order < recent.lastIpcObject.focusHistoryID))
                recent = window;
        }
        return recent || windows[0] || null;
    }

    // 平铺窗口按实际列与纵向位置排列；浮动窗口作为独立入口接在最后。
    readonly property var layout: {
        const tiled = windows.filter(window => !window.lastIpcObject.floating);
        tiled.sort((a, b) => a.lastIpcObject.at[0] - b.lastIpcObject.at[0] || a.lastIpcObject.at[1] - b.lastIpcObject.at[1]);
        const columns = [];
        let lastX = null;
        for (const window of tiled) {
            const x = window.lastIpcObject.at[0];
            if (x !== lastX) {
                columns.push([]);
                lastX = x;
            }
            columns[columns.length - 1].push(window);
        }
        for (const window of windows.filter(window => window.lastIpcObject.floating))
            columns.push([window]);

        const positions = {};
        columns.forEach((column, columnIndex) => {
            const gap = Math.min(2, 14 / column.length);
            const height = (28 - (column.length - 1) * gap) / column.length;
            column.forEach((window, rowIndex) => {
                positions[window.address] = {
                    x: columnIndex * 27,
                    y: rowIndex * (height + gap),
                    height: height,
                    stacked: column.length > 1,
                    column: columnIndex + 1
                };
            });
        });
        return {
            positions: positions,
            columns: columns.length
        };
    }
}
