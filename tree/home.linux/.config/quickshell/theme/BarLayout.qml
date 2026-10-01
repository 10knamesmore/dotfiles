pragma Singleton

import QtQuick

// Bar 布局 —「分组悬浮」：整栏无底板，左/中区散装模块，右区模块按组装进药丸
QtObject {
    readonly property int spacing: 8
    readonly property int sideMargin: 6
    readonly property int topMargin: 6
    readonly property bool moduleFlat: true

    // widget 项支持：字符串 id / {id, props} / {group: [...]}
    readonly property var leftWidgets: ["workspaces", "scrollstatus", "windowtitle", "tray"]
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
        "group": ["audio", "network"]
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
