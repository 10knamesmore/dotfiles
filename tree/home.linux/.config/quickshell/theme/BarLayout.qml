pragma Singleton

import QtQuick

// 顶栏布局：左侧导航信息带，中部独立模块，右侧按功能分组。
QtObject {
    readonly property int spacing: 8

    // widget 项支持：字符串 id / {id, props} / {group: [...]}
    readonly property var leftWidgets: ["navigation", "tray"]
    readonly property var centerWidgets: ["media"]
    readonly property var rightWidgets: [
        {
            "group": ["cpu", "memory", "netspeed"]
        },
        "controlCenter", "battery", "clock"]

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
