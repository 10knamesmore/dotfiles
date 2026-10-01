pragma Singleton

import QtQuick

// 顶栏布局：左侧导航信息带，中部独立模块，右侧按功能分组。
QtObject {
    readonly property int spacing: 8
    readonly property int sideMargin: 6
    readonly property int topMargin: 6
    readonly property bool moduleFlat: true

    // widget 项支持：字符串 id / {id, props} / {group: [...]}
    readonly property var leftWidgets: ["navigation", "tray"]
    readonly property var centerWidgets: [{
        "id": "netspeed",
        "props": {
            "direction": "up"
        }
    }, "media", "clock", {
        "id": "netspeed",
        "props": {
            "direction": "down"
        }
    }]
    readonly property var rightWidgets: [{
        "group": ["cpu", "memory"]
    }, {
        "group": ["audio", "network", "bluetooth"]
    }, {
        "group": ["clipboard", "notification", "screeneffects"]
    }, "battery"]

    // 归一化 widget 项 → {id, props}
    function normalize(item) {
        if (typeof item === "string")
            return {
                "id": item,
                "props": {}
            };
        return {
            "id": item.id,
            "props": item.props || {}
        };
    }
}
